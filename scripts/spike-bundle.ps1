# Vérifie le budget bundle du spike T7-00 (Yrs) : delta < 900 Kio brut.
# Usage : powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\spike-bundle.ps1
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$target = "wasm32-unknown-unknown"
foreach ($example in @("spike_baseline", "spike_wasm", "spike_wasm_schema")) {
    cargo build -q -p archifier-collab --example $example --target $target --release
    if ($LASTEXITCODE -ne 0) { throw "échec de compilation de $example" }
}

$dir = Join-Path $root "target/$target/release/examples"

function Get-WasmSize([string]$name) {
    $path = Join-Path $dir "$name.wasm"
    if (-not (Test-Path $path)) {
        $candidate = Get-ChildItem -Path $dir -Filter "$name-*.wasm" |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($null -eq $candidate) { throw "binaire introuvable : $name" }
        $path = $candidate.FullName
    }
    return (Get-Item $path).Length
}

$baseline = Get-WasmSize "spike_baseline"
$text = Get-WasmSize "spike_wasm"
$schema = Get-WasmSize "spike_wasm_schema"
$limit = 900 * 1024

$textDelta = $text - $baseline
$schemaDelta = $schema - $baseline

Write-Host ("baseline       : {0,9} o" -f $baseline)
Write-Host ("Y.Text         : {0,9} o  (delta {1,9} o)" -f $text, $textDelta)
Write-Host ("schéma C-01    : {0,9} o  (delta {1,9} o)" -f $schema, $schemaDelta)
Write-Host ("limite         : {0,9} o" -f $limit)

if ($textDelta -ge $limit -or $schemaDelta -ge $limit) {
    throw "budget bundle dépassé : delta Y.Text=$textDelta, schéma=$schemaDelta, limite=$limit"
}

Write-Host "OK : spike_bundle_delta_under_900kb"
