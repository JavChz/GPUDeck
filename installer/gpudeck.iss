#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef AppTag
  #define AppTag "v0.1.0"
#endif

[Setup]
AppId={{9F4E7C1A-6B2D-4C8E-9A51-3E7D2B8C4F10}}
AppName=GPUDeck
AppVersion={#AppVersion}
AppVerName=GPUDeck v{#AppVersion}
AppPublisher=Javier Garcia
AppPublisherURL=https://github.com/JavChz/GPUDeck
AppSupportURL=https://github.com/JavChz/GPUDeck/issues
AppUpdatesURL=https://github.com/JavChz/GPUDeck/releases
DefaultDirName={autopf}\GPUDeck
DefaultGroupName=GPUDeck
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\release
OutputBaseFilename=GPUDeck-Setup-{#AppTag}-x86_64
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=force
UninstallDisplayName=GPUDeck
UninstallDisplayIcon={app}\gpudeck.exe

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\gpudeck.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\fonts\OFL.txt"; DestDir: "{app}\assets\fonts"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\GPUDeck"; Filename: "{app}\gpudeck.exe"
Name: "{autodesktop}\GPUDeck"; Filename: "{app}\gpudeck.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\gpudeck.exe"; Description: "{cm:LaunchProgram,GPUDeck}"; Flags: nowait postinstall skipifsilent
