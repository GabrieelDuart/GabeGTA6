# Starts the title with settings for a small PC: GeForce GTX 1060 6 GB, 32 GB of RAM,
# an 8-core Ryzen 7 2700, Windows 10.
#   scripts\play-1060.ps1 -Exe <path of your own copy's executable>
#   scripts\play-1060.ps1 -Shaders <shader files>   translates them ahead, as the run will
# -Scale     the title's render scale, 0.5 to 1.0 (2/3 without it): 0.5 is the lightest
# -MipDrop   large textures start this many levels down, 0 to 3 (2): 3 is the lightest, 0 the sharpest
# -Hud       frame rate, frame times, VRAM, RAM and CPU on the picture (F3 shows and hides it)
# -Safe      without the newer, faster ways (the first thing to try when something is wrong)
# -High      a 1920x1080 display instead of 1280x720 (more of the card's 6 GB)
# -SeriesX   the Series X profile instead of Series S (needs more memory than a 6 GB card has)
# -Build     target\diag (default) or target\release
# -AutoEnter presses Enter (the A button) for you at 6, 8, 10, 12 and 15 s, past the
#            user screen ("Player (A)"); diag build only (VICTORY_KEYS), works without focus
# -More     other settings: -More @{VICTORY_PANEL='1'}
#
# What differs from scripts\play.ps1 (made for a 24 GB RX 7900 XTX and 64 GB of RAM):
# - the Series S profile and a 720p display: the title renders less and keeps its own
#   upscaler, so its targets fit a 6 GB card;
# - less of the card is kept for render targets (768 MB, not 3 GB), copies of title
#   buffers get 1 GB and large textures start 2 levels down, and textures are staged 64 MB and 8 ms a frame at most, and a frame waits 4 ms for pipelines, and textures stop at 85 % of the card's budget;
# - six translating threads instead of eight (eight cores here, the title needs some);
# - on Windows 10 the DirectX 12 Agility SDK must sit in target\<build>\D3D12 (see
#   docs/gtx1060.md): without it no pipeline can be made and the screen stays black.
# The GTX 1060 has no mesh shaders: its mesh programs run as compute programs whose
# triangles a vertex shader draws (docs/gtx1060.md, VICTORY_MESH_OUT_MB for the buffer).
param(
    [string]$Exe,
    [switch]$Safe,
    [switch]$High,
    [switch]$SeriesX,
    [double]$Scale = 0,
    [ValidateRange(0, 3)][int]$MipDrop = 2,
    [string[]]$Shaders = @(),
    [switch]$Hud,
    [switch]$AutoEnter,
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
    # Copies of the title's buffers in the card's memory: 1,024 MB, and 768 MB of its other
    # memory windows. Read over the bus instead, the frame waited for the card: a GTX 1660
    # 6 GB went from ~280 to ~100 ms a frame with these and the mip drop below
    # (Impre-visible/victory-gtx1660); an RTX 3070 8 GB from 327 to 61 ms with 2,048
    # (grimygunnz/nvidia-3070-VI).
    VICTORY_CARD_BUFFERS_MB = '1024'
    VICTORY_CARD_WINDOWS_MB = '768'
    # Like a low texture setting: host copies of large textures (1024 and up) start
    # -MipDrop levels down. Softer textures, far less of the card's memory.
    VICTORY_TEXTURE_MIP_DROP = "$MipDrop"
    # The title's output is 1280x720 (VICTORY_DISPLAY_MODE alone changes nothing: the title
    # takes it from its settings), drawn by its own upscaler, not FidelityFX's (whose scale
    # cannot be lowered safely). Byte-checked: another build of the title is left alone.
    VICTORY_OUTPUT = '1280x720'
    VICTORY_UPSCALER = '6'
    VICTORY_TEXTURE_ROOM_PERCENT = '85'
    # Textures staged a frame: at most 64 MB, in a slice of 8 ms of each frame that all
    # its passes share (the defaults, 192 MB and 250 ms a pass, can take every frame;
    # kelvaroq's patch 0015 and its profile for small cards).
    VICTORY_TEXTURE_STAGE_MB = '64'
    VICTORY_TEXTURE_STAGE_PER_FRAME = '1'
    VICTORY_TEXTURE_STAGE_MS = '8'
    # A frame waits 4 ms at most for pipelines still being compiled (500 by default):
    # their draws are left out until they are ready, the decoration bake still gets
    # its 500 ms (kelvaroq's patch 0016).
    VICTORY_PIPELINE_WAIT_MS = '4'
    # The ray-tracing structures are not built (VICTORY_BVH_BUILDER): the ~1,000 programs
    # that shade their hits are not compiled (from codec-d/cool_fox via cgbj-reupload).
    VICTORY_RT_RESOLVE = '0'
    VICTORY_ACTIVATE_RAYS = '0'
    # Ray walks end at once instead of walking structures never built (~10 ms a frame on
    # an RTX 5070; trustno1-coder/VIctory-NVIDIA). Part of the shader cache key.
    VICTORY_BVH_MISS = '1'
    # NVIDIA's view heap holds 1,000,000 descriptors, the title wants 1,245,184: more of
    # them for tables made a draw at a time, or thousands of draws a frame are dropped.
    VICTORY_DYNAMIC_SLOTS = '524288'
    # 32 GB of RAM: what the card and the title's memory are kept to is made for 32 GB
    # (the run uses ~35 GB: give Windows a page file, docs/gtx1060.md).
    VICTORY_RESIDENCY = '1'
}
if ($High) { $settings.VICTORY_DISPLAY_MODE = '1920x1080'; $settings.VICTORY_OUTPUT = '1920x1080' }
if ($Hud) { $settings.VICTORY_PANEL = '1' }
if ($AutoEnter) { $settings.VICTORY_KEYS = '6:Enter,8:Enter,10:Enter,12:Enter,15:Enter' }
if ($Scale -gt 0) { $settings.VICTORY_RENDER_SCALE = "$Scale" }
# The ~1,000 ray-tracing resolves and 34 traced shadows are not compiled nor drawn (their
# structures are never built on NVIDIA: rooms went white; trustno1-coder/VIctory-NVIDIA).
$rtList = Join-Path $PSScriptRoot 'rt_off_hashes.txt'
if (Test-Path $rtList) { $settings.VICTORY_SKIP_PIPELINES = (Get-Content $rtList -Raw).Trim() }
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_THIN_THREADS = '6'; $settings.VICTORY_JOIN_PARTS = '6'
    $settings.VICTORY_UPLOAD_AHEAD = '1'
    # Less work for the processor each frame: the constants heap is read where the title
    # writes it (no 174 MB of uploads), and descriptor stores say what they wrote (no
    # scan of ~600,000 slots).
    $settings.VICTORY_PORT_CONSTANTS = '1'; $settings.VICTORY_PORT_DESCRIPTORS = '1'
}
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }
$console = if ($SeriesX) { 'series-x' } else { 'series-s' }

$root = Join-Path $PSScriptRoot '..'
$victory = Join-Path $root "target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (docs/gtx1060.md)."; return }
if ($Shaders.Count -eq 0 -and -not ($Exe -and (Test-Path $Exe))) { Write-Host "No -Exe '$Exe'."; return }
# Not $build: PowerShell names ignore case, so it would be the -Build parameter (diag/release)
# and the assignment fails its ValidateSet, which also skipped this Agility SDK check.
$osBuild = [Environment]::OSVersion.Version.Build
$agility = Join-Path (Split-Path $victory) 'D3D12\D3D12Core.dll'
if ($osBuild -lt 22000 -and -not (Test-Path $agility)) {
    Write-Host "Windows 10 without $agility : the screen will stay black. See docs/gtx1060.md."
    return
}
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
# The caches (translated shaders, pipelines: several GB) and saves go next to the
# repository, not to %LOCALAPPDATA% on C:, unless you chose a place.
$store = Join-Path (Split-Path (Resolve-Path $root).Path) 'victory-data'
if (-not $env:VICTORY_CACHE_DIR) { $env:VICTORY_CACHE_DIR = Join-Path $store 'cache' }
if (-not $env:VICTORY_DATA_DIR) { $env:VICTORY_DATA_DIR = Join-Path $store 'data' }
Write-Host "Caches in $env:VICTORY_CACHE_DIR"
if ($Shaders.Count -gt 0) {
    # The same settings as the run: its translations are the ones looked up.
    $Shaders = @($Shaders | ForEach-Object { (Resolve-Path $_).Path })
    Push-Location $root
    try { & $victory shaders @Shaders } finally { Pop-Location }
    return
}
$Exe = (Resolve-Path $Exe).Path
Push-Location $root   # victory.toml is looked for here
try { & $victory run $Exe --execute --console $console } finally { Pop-Location }
