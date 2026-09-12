# Verification complete du depot.
#
# Format, clippy sans indulgence, tests Rust et verification TypeScript.
# C'est ce qui doit passer avant tout commit.
#
#   powershell -File scripts/verifier.ps1

$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

$echecs = @()

function Invoke-Etape {
    param(
        [string] $Nom,
        [scriptblock] $Action
    )
    Write-Host ""
    Write-Host "-> $Nom" -ForegroundColor Cyan
    & $Action
    if ($LASTEXITCODE -ne 0) {
        $script:echecs += $Nom
        Write-Host "   echec ($LASTEXITCODE)" -ForegroundColor Red
    }
    else {
        Write-Host "   ok" -ForegroundColor Green
    }
}

Invoke-Etape "Format Rust" { cargo fmt --all --check }
Invoke-Etape "Clippy (aucun avertissement tolere)" {
    cargo clippy --workspace --all-targets --all-features -- -D warnings
}
Invoke-Etape "Tests Rust" { cargo test --workspace }
Invoke-Etape "Types TypeScript" { npm.cmd run check --silent }

Write-Host ""
if ($echecs.Count -eq 0) {
    Write-Host "Tout est vert." -ForegroundColor Green
    exit 0
}

Write-Host "Etapes en echec :" -ForegroundColor Red
foreach ($echec in $echecs) { Write-Host "  - $echec" -ForegroundColor Red }
exit 1
