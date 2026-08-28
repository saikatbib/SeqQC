#define MyAppName "SeqQC Studio"
#define MyAppPublisher "Saikat Basak and SeqQC contributors"
#define MyAppURL "https://github.com/saikatbib/SeqQC"
#define MyAppExeName "SeqQC Studio.exe"

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0-alpha.2"
#endif

#ifndef MyNumericVersion
  #define MyNumericVersion "0.1.0.2"
#endif

#ifndef MyOutputBaseFilename
  #define MyOutputBaseFilename "SeqQC-Studio-Setup-v0.1.0-alpha.2-windows-x86_64"
#endif

[Setup]
AppId={{F4F594CA-0486-4475-A998-E5A06A523F49}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
AppCopyright=Copyright (C) 2026 Saikat Basak and SeqQC contributors
VersionInfoVersion={#MyNumericVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=Offline sequencing quality-control studio
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyNumericVersion}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir=..\..\dist
OutputBaseFilename={#MyOutputBaseFilename}
SetupIconFile=..\..\apps\desktop\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\SeqQC Studio.ico
UninstallDisplayName={#MyAppName}
LicenseFile=..\..\LICENSE
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
SetupLogging=yes
CloseApplications=yes
RestartApplications=no
Uninstallable=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "..\..\target\release\seqqc-native-desktop.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion
Source: "..\..\apps\desktop\src-tauri\icons\icon.ico"; DestDir: "{app}"; DestName: "SeqQC Studio.ico"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\PORTABLE_PACKAGE_README.txt"; DestDir: "{app}"; DestName: "README.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\SeqQC Studio.ico"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\SeqQC Studio.ico"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent
