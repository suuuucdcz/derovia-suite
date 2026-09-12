# Documentation Rust hors-ligne.
#
# Tout est deja sur le disque : le composant rust-docs installe avec la
# toolchain contient les quinze livres officiels, et cargo doc genere l'API de
# nos crates et de toutes leurs dependances. Aucune connexion necessaire.
#
#   powershell -File scripts/docs.ps1            # liste ce qui est disponible
#   powershell -File scripts/docs.ps1 -Ouvrir book
#   powershell -File scripts/docs.ps1 -Regenerer # relance cargo doc

param(
    [string] $Ouvrir = "",
    [switch] $Regenerer
)

$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

$toolchain = (rustc --print sysroot).Trim()
$livres = Join-Path $toolchain "share/doc/rust/html"
$api = Join-Path $racine "target/doc"

# Les quinze documents fournis avec la toolchain.
$catalogue = [ordered]@{
    "std"           = "std/index.html"
    "core"          = "core/index.html"
    "alloc"         = "alloc/index.html"
    "book"          = "book/index.html"
    "reference"     = "reference/index.html"
    "by-example"    = "rust-by-example/index.html"
    "nomicon"       = "nomicon/index.html"
    "cargo"         = "cargo/index.html"
    "clippy"        = "clippy/index.html"
    "rustdoc"       = "rustdoc/index.html"
    "edition"       = "edition-guide/index.html"
    "style"         = "style-guide/index.html"
    "embedded"      = "embedded-book/index.html"
    "unstable"      = "unstable-book/index.html"
    "erreurs"       = "error_codes/index.html"
}

if ($Regenerer) {
    Write-Host "Generation de la documentation API (workspace + dependances)..." -ForegroundColor Cyan
    cargo doc --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($Ouvrir -ne "") {
    if ($Ouvrir -eq "api") {
        $cible = Join-Path $api "derovia_arbitrage/index.html"
    }
    elseif ($catalogue.Contains($Ouvrir)) {
        $cible = Join-Path $livres $catalogue[$Ouvrir]
    }
    else {
        Write-Host "Document inconnu : $Ouvrir" -ForegroundColor Red
        Write-Host "Disponibles : api, $($catalogue.Keys -join ', ')"
        exit 1
    }

    if (-not (Test-Path $cible)) {
        Write-Host "Introuvable : $cible" -ForegroundColor Red
        if ($Ouvrir -eq "api") { Write-Host "Lancer d'abord : scripts/docs.ps1 -Regenerer" }
        exit 1
    }
    Start-Process $cible
    exit 0
}

Write-Host ""
Write-Host "Documentation Rust hors-ligne" -ForegroundColor Cyan
Write-Host "  toolchain : $livres"
Write-Host ""

foreach ($cle in $catalogue.Keys) {
    $chemin = Join-Path $livres $catalogue[$cle]
    if (Test-Path $chemin) {
        Write-Host ("  {0,-12} disponible" -f $cle) -ForegroundColor Green
    }
    else {
        Write-Host ("  {0,-12} absent (rustup component add rust-docs)" -f $cle) -ForegroundColor DarkYellow
    }
}

Write-Host ""
if (Test-Path (Join-Path $api "derovia_arbitrage/index.html")) {
    Write-Host "  api          disponible ($api)" -ForegroundColor Green
}
else {
    Write-Host "  api          absent (scripts/docs.ps1 -Regenerer)" -ForegroundColor DarkYellow
}

Write-Host ""
Write-Host "Ouvrir :  powershell -File scripts/docs.ps1 -Ouvrir book" -ForegroundColor DarkGray
Write-Host ""
