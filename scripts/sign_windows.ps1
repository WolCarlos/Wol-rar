param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [string]$PfxPath,
    [string]$PfxPassword,
    [string]$Subject = "Wol-rar Local Development",
    [switch]$CreateSelfSigned
)

$ErrorActionPreference = "Stop"
if ($PfxPath) {
    $securePassword = ConvertTo-SecureString $PfxPassword -AsPlainText -Force
    $certificate = Get-PfxCertificate -FilePath $PfxPath -Password $securePassword
} else {
    $certificate = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
        Where-Object { $_.Subject -eq "CN=$Subject" -and $_.HasPrivateKey } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1
    if (-not $certificate -and $CreateSelfSigned) {
        $certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=$Subject" -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddYears(2)
    }
}
if (-not $certificate) {
    throw "Nessun certificato di code signing disponibile."
}
$result = Set-AuthenticodeSignature -FilePath $FilePath -Certificate $certificate -HashAlgorithm SHA256
if ($result.Status -notin @("Valid", "UnknownError")) {
    throw "Firma Authenticode non riuscita: $($result.Status) $($result.StatusMessage)"
}
$verify = Get-AuthenticodeSignature -FilePath $FilePath
[PSCustomObject]@{
    File = $FilePath
    Subject = $verify.SignerCertificate.Subject
    Thumbprint = $verify.SignerCertificate.Thumbprint
    SignaturePresent = ($null -ne $verify.SignerCertificate)
    TrustStatus = $verify.Status
} | ConvertTo-Json
