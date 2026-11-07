#ifndef AppVersion
#define AppVersion "0.3.0"
#endif

[Setup]
AppId={{6B8F8F60-93B7-4B58-9B9C-7A11C0DE0003}
AppName=Wol-rar
AppVersion={#AppVersion}
AppVerName=Wol-rar {#AppVersion}
AppPublisher=WolCarlos
AppPublisherURL=https://github.com/WolCarlos/Wol-rar
AppSupportURL=https://github.com/WolCarlos/Wol-rar/issues
AppUpdatesURL=https://github.com/WolCarlos/Wol-rar/releases
AppComments=Archiviatore privato, locale e senza telemetria
DefaultDirName={localappdata}\Programs\Wol-rar
DefaultGroupName=Wol-rar
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\release
OutputBaseFilename=Wol-rar-Setup-x64
SetupIconFile=..\assets\wol-rar.ico
UninstallDisplayIcon={app}\Wol-rar.exe
UninstallDisplayName=Wol-rar
VersionInfoVersion={#AppVersion}.0
VersionInfoCompany=WolCarlos
VersionInfoDescription=Installer di Wol-rar
VersionInfoProductName=Wol-rar
VersionInfoProductVersion={#AppVersion}.0
WizardStyle=modern
WizardSizePercent=110
CloseApplications=yes
RestartApplications=no
Compression=lzma2/ultra64
SolidCompression=yes
InternalCompressLevel=ultra64

[Tasks]
Name: "desktopicon"; Description: "Crea un collegamento sul desktop"; GroupDescription: "Collegamenti:"; Flags: unchecked
Name: "contextmenu"; Description: "Aggiungi il menu Wol-rar al tasto destro di Explorer"; GroupDescription: "Integrazione Explorer:"

[Files]
Source: "..\dist\Wol-rar.exe"; DestDir: "{app}"; Flags: ignoreversion signcheck
Source: "..\7ZIP-LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\wol-rar.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\menu-*.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\menu-*.png"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{group}\Wol-rar"; Filename: "{app}\Wol-rar.exe"; IconFilename: "{app}\assets\wol-rar.ico"
Name: "{group}\Novità e download"; Filename: "https://github.com/WolCarlos/Wol-rar/releases"; IconFilename: "{app}\assets\menu-update.ico"
Name: "{group}\Disinstalla Wol-rar"; Filename: "{uninstallexe}"; IconFilename: "{app}\assets\wol-rar.ico"
Name: "{autodesktop}\Wol-rar"; Filename: "{app}\Wol-rar.exe"; IconFilename: "{app}\assets\wol-rar.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\Wol-rar.exe"; Description: "Avvia Wol-rar"; Flags: nowait postinstall skipifsilent unchecked

[Registry]
; Archivi: un solo contenitore con azioni pertinenti.
#define ArchiveExtension ".7z"
#sub ArchiveMenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Wol-rar"; Tasks: contextmenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\wol-rar.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar"; ValueType: string; ValueName: "Position"; ValueData: "Middle"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar"; ValueType: string; ValueName: "SubCommands"; ValueData: ""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\01Apri"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Apri con Wol-rar"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\01Apri"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\menu-open.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\01Apri\command"; ValueType: string; ValueData: """{app}\Wol-rar.exe"" --open ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\02Estrai"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Estrai in una nuova cartella"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\02Estrai"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\menu-extract.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\02Estrai\command"; ValueType: string; ValueData: """{app}\Wol-rar.exe"" --extract ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\03Verifica"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Verifica integrità"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\03Verifica"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\menu-test.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\{#ArchiveExtension}\shell\Wol-rar\shell\03Verifica\command"; ValueType: string; ValueData: """{app}\Wol-rar.exe"" --test ""%1"""; Tasks: contextmenu
#endsub
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".zip"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".rar"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".tar"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".gz"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".bz2"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".xz"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".iso"
#emit ArchiveMenu
#undef ArchiveExtension
#define ArchiveExtension ".cab"
#emit ArchiveMenu

; File generici e cartelle: soltanto creazione archivio, per non affollare il menu.
#define ShellTarget "*"
#sub CompressMenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Wol-rar"; Tasks: contextmenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\wol-rar.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar"; ValueType: string; ValueName: "Position"; ValueData: "Middle"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar"; ValueType: string; ValueName: "SubCommands"; ValueData: ""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar\shell\01Comprimi"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Comprimi in un archivio 7z o ZIP"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar\shell\01Comprimi"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\menu-create.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\{#ShellTarget}\shell\Wol-rar\shell\01Comprimi\command"; ValueType: string; ValueData: """{app}\Wol-rar.exe"" --create ""%1"""; Tasks: contextmenu
#endsub
#emit CompressMenu
#undef ShellTarget
#define ShellTarget "Directory"
#emit CompressMenu

; Click destro nel contenuto di una cartella.
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Wol-rar"; Tasks: contextmenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\wol-rar.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar"; ValueType: string; ValueName: "Position"; ValueData: "Middle"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar"; ValueType: string; ValueName: "SubCommands"; ValueData: ""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar\shell\01Comprimi"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Comprimi il contenuto di questa cartella"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar\shell\01Comprimi"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\assets\menu-create.ico"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Wol-rar\shell\01Comprimi\command"; ValueType: string; ValueData: """{app}\Wol-rar.exe"" --create ""%V"""; Tasks: contextmenu
