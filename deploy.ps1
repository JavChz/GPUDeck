<#
.SYNOPSIS
    Automates GPUDeck versioning, verification, tagging, and GitHub Release deployment.

.EXAMPLES
    .\deploy.ps1 current          # Release the current Cargo.toml version (e.g. 0.2.0 -> v0.2.0)
    .\deploy.ps1 patch            # Bump patch version (e.g. 0.2.0 -> 0.2.1), test, commit, tag & push
    .\deploy.ps1 minor            # Bump minor version (e.g. 0.2.0 -> 0.3.0), test, commit, tag & push
    .\deploy.ps1 major            # Bump major version (e.g. 0.2.0 -> 1.0.0), test, commit, tag & push
    .\deploy.ps1 0.3.0-beta.1     # Set an explicit SemVer version (publishes as GitHub Pre-release)
    .\deploy.ps1 patch -DryRun    # Preview all changes and commands without modifying anything
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Version = "current",

    [switch]$DryRun,
    [switch]$SkipTests,
    [switch]$NoPush,
    [switch]$AllowAnyBranch
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Exe,

        [string[]]$CommandArgs = @()
    )
    Write-Host ">> $Exe $($CommandArgs -join ' ')" -ForegroundColor DarkCyan
    & $Exe @CommandArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code $($LASTEXITCODE): $Exe $($CommandArgs -join ' ')"
    }
}

function Get-CargoPackageVersion {
    param([string]$CargoTomlPath)
    $lines = [System.IO.File]::ReadAllLines($CargoTomlPath, [System.Text.UTF8Encoding]::new($false))
    $inPackage = $false
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[(.+)\]$') {
            $inPackage = ($matches[1].Trim() -eq "package")
            continue
        }
        if ($inPackage -and $trimmed -match '^version\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
    }
    throw "Could not find [package] version in $CargoTomlPath"
}

function Set-CargoPackageVersion {
    param(
        [string]$CargoTomlPath,
        [string]$NewVersion
    )
    $raw = [System.IO.File]::ReadAllText($CargoTomlPath, [System.Text.UTF8Encoding]::new($false))
    $eol = "`n"
    if ($raw.Contains("`r`n")) {
        $eol = "`r`n"
    }
    $lines = $raw -split "\r?\n", -1
    $inPackage = $false
    $updated = $false
    for ($i = 0; $i -lt $lines.Length; $i++) {
        $trimmed = $lines[$i].Trim()
        if ($trimmed -match '^\[(.+)\]$') {
            $inPackage = ($matches[1].Trim() -eq "package")
            continue
        }
        if ($inPackage -and -not $updated -and $lines[$i] -match '^(\s*version\s*=\s*)"([^"]+)"(.*)$') {
            $lines[$i] = $matches[1] + '"' + $NewVersion + '"' + $matches[3]
            $updated = $true
        }
    }
    if (-not $updated) {
        throw "Failed to update [package] version in $CargoTomlPath"
    }
    [System.IO.File]::WriteAllText($CargoTomlPath, ($lines -join $eol), [System.Text.UTF8Encoding]::new($false))
}

function Resolve-TargetVersion {
    param(
        [string]$Current,
        [string]$Requested
    )
    $req = $Requested.Trim()
    $lower = $req.ToLowerInvariant()
    if ($lower -eq "current") {
        if ($Current -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
            throw "Current Cargo.toml version '$Current' is not valid SemVer."
        }
        return $Current
    }

    if ($lower -in @("patch", "minor", "major")) {
        if ($Current -notmatch '^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?$') {
            throw "Cannot bump non-SemVer version '$Current'."
        }
        $maj = [int]$matches[1]
        $min = [int]$matches[2]
        $pat = [int]$matches[3]
        switch ($lower) {
            "major" { return "$($maj + 1).0.0" }
            "minor" { return "$maj.$($min + 1).0" }
            "patch" { return "$maj.$min.$($pat + 1)" }
        }
    }

    $stripped = $req
    if ($stripped.StartsWith("v") -or $stripped.StartsWith("V")) {
        $stripped = $stripped.Substring(1)
    }
    if ($stripped -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
        throw "Invalid version '$Requested'. Use current, patch, minor, major, or SemVer X.Y.Z (optional -prerelease)."
    }
    return $stripped
}

function Update-ChangelogForRelease {
    param(
        [string]$ChangelogPath,
        [string]$TargetVersion,
        [string]$PreviousTag,
        [string]$ReleaseDate
    )
    $raw = [System.IO.File]::ReadAllText($ChangelogPath, [System.Text.UTF8Encoding]::new($false))
    $escapedVer = [regex]::Escape($TargetVersion)
    if ($raw -match "(?m)^## \[$escapedVer\]") {
        Write-Host "CHANGELOG.md already has section [$TargetVersion]; skipping changelog edit." -ForegroundColor Yellow
        return $false
    }

    $eol = "`n"
    if ($raw.Contains("`r`n")) {
        $eol = "`r`n"
    }

    $tagName = "v$TargetVersion"
    $newText = $raw
    $headingRegex = [regex]'(?m)^## \[Unreleased\]\s*$'
    if ($headingRegex.IsMatch($newText)) {
        $replacement = "## [Unreleased]$eol$eol## [$TargetVersion] - $ReleaseDate"
        $newText = $headingRegex.Replace($newText, $replacement, 1)
    } else {
        throw "CHANGELOG.md is missing an '## [Unreleased]' heading."
    }

    if ([string]::IsNullOrWhiteSpace($PreviousTag)) {
        $versionLinkLine = "[$TargetVersion]: https://github.com/JavChz/GPUDeck/releases/tag/$tagName"
    } else {
        $versionLinkLine = "[$TargetVersion]: https://github.com/JavChz/GPUDeck/compare/$PreviousTag...$tagName"
    }
    $unreleasedLinkLine = "[Unreleased]: https://github.com/JavChz/GPUDeck/compare/$tagName...HEAD"

    $unreleasedLinkRegex = [regex]'(?m)^\[Unreleased\]:.*$'
    if ($unreleasedLinkRegex.IsMatch($newText)) {
        $newText = $unreleasedLinkRegex.Replace($newText, "$unreleasedLinkLine$eol$versionLinkLine", 1)
    } else {
        $newText = $newText.TrimEnd() + "$eol$eol$unreleasedLinkLine$eol$versionLinkLine$eol"
    }

    [System.IO.File]::WriteAllText($ChangelogPath, $newText, [System.Text.UTF8Encoding]::new($false))
    return $true
}

Push-Location $PSScriptRoot
try {
    $cargoPath = Join-Path $PSScriptRoot "Cargo.toml"
    $changelogPath = Join-Path $PSScriptRoot "CHANGELOG.md"

    $currentVersion = Get-CargoPackageVersion -CargoTomlPath $cargoPath
    $targetVersion = Resolve-TargetVersion -Current $currentVersion -Requested $Version
    $tagName = "v$targetVersion"
    $today = (Get-Date).ToString("yyyy-MM-dd")

    # Verify branch
    $branch = (& git rev-parse --abbrev-ref HEAD).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to determine current git branch."
    }
    if (-not $AllowAnyBranch -and $branch -ne "main") {
        throw "Current branch is '$branch'. Switch to 'main' or pass -AllowAnyBranch to deploy from another branch."
    }

    # Check if tag already exists locally or on origin
    $localTag = (& git tag -l $tagName | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to check local git tags."
    }
    if (-not [string]::IsNullOrEmpty($localTag)) {
        throw "Tag '$tagName' already exists locally. Pick a higher version (current Cargo.toml: $currentVersion)."
    }

    $remoteTag = (& git ls-remote --tags origin "refs/tags/$tagName" | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to check tags on 'origin'. Ensure network and git remote access are available."
    }
    if (-not [string]::IsNullOrEmpty($remoteTag)) {
        throw "Tag '$tagName' already exists on origin. Pick a higher version."
    }

    $prevTag = (& git tag -l "v*" --sort=-v:refname | Select-Object -First 1 | Out-String).Trim()
    $willUpdateCargo = ($targetVersion -ne $currentVersion)
    $willUpdateChangelog = $false
    if (Test-Path $changelogPath) {
        $clContent = [System.IO.File]::ReadAllText($changelogPath, [System.Text.UTF8Encoding]::new($false))
        $escapedVer = [regex]::Escape($targetVersion)
        if ($clContent -notmatch "(?m)^## \[$escapedVer\]") {
            $willUpdateChangelog = $true
        }
    }

    Write-Host ""
    Write-Host "GPUDeck Release Plan" -ForegroundColor Cyan
    Write-Host "  Branch          : $branch"
    Write-Host "  Current version : $currentVersion"
    Write-Host "  Target version  : $targetVersion (tag: $tagName)"
    if ($willUpdateCargo) {
        Write-Host "  Cargo.toml      : update $currentVersion -> $targetVersion"
    } else {
        Write-Host "  Cargo.toml      : already at $targetVersion"
    }
    if ($willUpdateChangelog) {
        $prevDisplay = if ([string]::IsNullOrWhiteSpace($prevTag)) { "none" } else { $prevTag }
        Write-Host "  CHANGELOG.md    : promote [Unreleased] -> [$targetVersion] - $today (prev tag: $prevDisplay)"
    } else {
        Write-Host "  CHANGELOG.md    : no change needed"
    }
    if ($SkipTests) {
        Write-Host "  Verification    : SKIPPED (-SkipTests); syncing Cargo.lock via cargo metadata"
    } else {
        Write-Host "  Verification    : cargo test + cargo test --locked + cargo build --release --locked"
    }
    if ($NoPush) {
        Write-Host "  Push            : SKIPPED (-NoPush); commit and tag created locally only"
    } else {
        Write-Host "  Push            : git push origin $branch + git push origin $tagName"
    }
    Write-Host ""

    if ($DryRun) {
        Write-Host "[DryRun] No files modified, nothing committed, tagged, or pushed." -ForegroundColor Yellow
        exit 0
    }

    if ($willUpdateCargo) {
        Set-CargoPackageVersion -CargoTomlPath $cargoPath -NewVersion $targetVersion
        Write-Host "Updated Cargo.toml to version $targetVersion" -ForegroundColor Green
    }

    if ($willUpdateChangelog) {
        $changedCl = Update-ChangelogForRelease `
            -ChangelogPath $changelogPath `
            -TargetVersion $targetVersion `
            -PreviousTag $prevTag `
            -ReleaseDate $today
        if ($changedCl) {
            Write-Host "Updated CHANGELOG.md with [$targetVersion] - $today" -ForegroundColor Green
        }
    }

    if ($SkipTests) {
        Invoke-Checked -Exe "cargo" -CommandArgs @("metadata", "--format-version", "1") | Out-Null
    } else {
        Invoke-Checked -Exe "cargo" -CommandArgs @("test")
        Invoke-Checked -Exe "cargo" -CommandArgs @("test", "--locked")
        Invoke-Checked -Exe "cargo" -CommandArgs @("build", "--release", "--locked")
    }

    Invoke-Checked -Exe "git" -CommandArgs @("add", "-A")
    $pendingChanges = (& git status --porcelain | Out-String).Trim()
    if (-not [string]::IsNullOrEmpty($pendingChanges)) {
        Invoke-Checked -Exe "git" -CommandArgs @("commit", "-m", "chore: release $tagName")
    } else {
        Write-Host "Working tree clean; no commit needed before tagging $tagName." -ForegroundColor Yellow
    }

    Invoke-Checked -Exe "git" -CommandArgs @("tag", "-a", $tagName, "-m", "GPUDeck $tagName")

    if ($NoPush) {
        Write-Host ""
        Write-Host "Local release $tagName ready (-NoPush). To publish later, run:" -ForegroundColor Green
        Write-Host "  git push origin $branch"
        Write-Host "  git push origin $tagName"
    } else {
        Invoke-Checked -Exe "git" -CommandArgs @("push", "origin", $branch)
        Invoke-Checked -Exe "git" -CommandArgs @("push", "origin", $tagName)
        Write-Host ""
        Write-Host "Deployed $tagName! GitHub Actions is building the Windows installer, Linux .deb, and archives." -ForegroundColor Green
        Write-Host "  Actions : https://github.com/JavChz/GPUDeck/actions"
        Write-Host "  Release : https://github.com/JavChz/GPUDeck/releases/tag/$tagName"
    }
}
finally {
    Pop-Location
}
