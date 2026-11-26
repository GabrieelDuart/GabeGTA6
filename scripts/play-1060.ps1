# Starts the title with settings for a small PC: GeForce GTX 1060 6 GB, 32 GB of RAM,
# an 8-core Ryzen 7 2700, Windows 10.
#   scripts\play-1060.ps1 -Exe <path of your own copy's executable>
# -Safe      without the newer, faster ways (the first thing to try when something is wrong)
# -High      a 1920x1080 display instead of 1280x720 (more of the card's 6 GB)
# -SeriesX   the Series X profile instead of Series S (needs more memory than a 6 GB card has)
# -Build     target\diag (default) or target\release
# -More      other settings: -More @{VICTORY_PANEL='1'}
#
# What differs from scripts\play.ps1 (made for a 24 GB RX 7900 XTX and 64 GB of RAM):
# - the Series S profile and a 720p display: the title renders less and keeps its own
#   upscaler, so its targets fit a 6 GB card;
# - less of the card is kept for render targets (768 MB, not 3 GB) and for copies of
#   title buffers (256 MB), and textures are staged 64 MB and 33 ms a frame at most, and textures stop at 85 % of the card's budget;
# - six translating threads instead of eight (eight cores here, the title needs some);
# - on Windows 10 the DirectX 12 Agility SDK must sit in target\<build>\D3D12 (see
#   docs/gtx1060.md): without it no pipeline can be made and the screen stays black.
# The GTX 1060 has no mesh shaders: its mesh programs run as compute programs whose
# triangles a vertex shader draws (docs/gtx1060.md, VICTORY_MESH_OUT_MB for the buffer).
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$High,
    [switch]$SeriesX,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{}
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.9'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
    # The small card.
    VICTORY_DISPLAY_MODE = '1280x720'
    VICTORY_CARD_ROOM_FOR_TARGETS_MB = '768'
    VICTORY_CARD_BUFFERS_MB = '256'
    VICTORY_TEXTURE_ROOM_PERCENT = '85'
    # Textures staged a frame: at most 64 MB and 33 ms (the defaults, 192 MB and 250 ms,
    # can take every frame; kelvaroq's profile for small cards uses 64 MB).
    VICTORY_TEXTURE_STAGE_MB = '64'
    VICTORY_TEXTURE_STAGE_MS = '33'
}
if ($High) { $settings.VICTORY_DISPLAY_MODE = '1920x1080' }
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_THIN_THREADS = '6'; $settings.VICTORY_JOIN_PARTS = '6'
    $settings.VICTORY_UPLOAD_AHEAD = '1'
}
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }
$console = if ($SeriesX) { 'series-x' } else { 'series-s' }

$root = Join-Path $PSScriptRoot '..'
$victory = Join-Path $root "target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (docs/gtx1060.md)."; return }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; return }
$build = [Environment]::OSVersion.Version.Build
$agility = Join-Path (Split-Path $victory) 'D3D12\D3D12Core.dll'
if ($build -lt 22000 -and -not (Test-Path $agility)) {
    Write-Host "Windows 10 without $agility : the screen will stay black. See docs/gtx1060.md."
    return
}
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$Exe = (Resolve-Path $Exe).Path
Push-Location $root   # victory.toml is looked for here
try { & $victory run $Exe --execute --console $console } finally { Pop-Location }
