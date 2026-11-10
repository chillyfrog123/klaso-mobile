$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargo) {
    & $cargo.Source run --release --locked
} else {
    $cargoPath = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (-not (Test-Path -LiteralPath $cargoPath)) {
        throw 'Rust is required. Install Rust, then run this launcher again.'
    }
    & $cargoPath run --release --locked
}
exit $LASTEXITCODE
