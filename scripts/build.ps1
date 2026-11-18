# Compile le front et produit le bundle web dans ./pkg
$ErrorActionPreference = "Stop"

cargo build -p archifier-ui --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { throw "échec de la compilation wasm" }

wasm-bindgen --target web --no-typescript --out-dir pkg `
  target/wasm32-unknown-unknown/release/archifier_ui.wasm
if ($LASTEXITCODE -ne 0) { throw "échec de wasm-bindgen" }

$size = (Get-Item pkg/archifier_ui_bg.wasm).Length / 1MB
Write-Host ("Bundle prêt : pkg/ ({0:N2} Mo de wasm)" -f $size)
Write-Host "Servir avec : node scripts/serve.mjs 4173"
