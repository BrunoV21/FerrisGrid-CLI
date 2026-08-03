param([Parameter(Mandatory = $true)][string]$FerrisGridBinary)

$ErrorActionPreference = "Stop"
$root = Join-Path ([System.IO.Path]::GetTempPath()) ("ferrisgrid-rust-e2e-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $root | Out-Null
$statePath = Join-Path $root "state.json"
$readyPath = Join-Path $root "ready"
$tracePath = Join-Path $root "traces"
$fixturePath = Join-Path $PSScriptRoot "windows-e2e-fixture.ps1"
$fixture = Start-Process powershell.exe -ArgumentList @("-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $fixturePath, "-StatePath", $statePath, "-ReadyPath", $readyPath) -PassThru

function Wait-For([scriptblock]$Condition, [int]$TimeoutSeconds = 15) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        try { if (& $Condition) { return } } catch { }
        Start-Sleep -Milliseconds 100
    }
    $fixtureState = if (Test-Path $statePath) { Get-Content -Raw -Encoding UTF8 $statePath } else { "unavailable" }
    throw "native Windows E2E condition timed out; fixture state: $fixtureState"
}

function Get-State { Get-Content -Raw -Encoding UTF8 $statePath | ConvertFrom-Json }
function To-Normalized([int]$Value, [int]$Origin, [int]$Size) { [Math]::Max(0, [Math]::Min(1000, [Math]::Round((($Value - $Origin) / [double]$Size) * 1000))) }
function Invoke-Ferris([string[]]$Arguments) {
    & $FerrisGridBinary @Arguments | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "FerrisGrid exited with ${LASTEXITCODE}: $($Arguments -join ' ')" }
}
function Invoke-Action([string]$Markdown) {
    $actionPath = Join-Path $root "action.md"
    [System.IO.File]::WriteAllText($actionPath, $Markdown, (New-Object System.Text.UTF8Encoding($false)))
    Invoke-Ferris @("act", "--backend", "native-windows", "--output-dir", $tracePath, "--session", "e2e", "--screen-id", "screen-1", "--file", $actionPath, "--format", "png", "--grid-overlay", "false", "--resolution", "fast")
}

try {
    Wait-For { Test-Path $readyPath } 30
    Invoke-Ferris @("doctor", "--backend", "native-windows", "--output-dir", $tracePath)
    $observation = & $FerrisGridBinary observe --backend native-windows --output-dir $tracePath --session e2e --format png --grid-overlay false --resolution fast
    if ($LASTEXITCODE -ne 0) { throw "native observation failed" }
    $observation | Out-Host
    $screenLine = [regex]::Match(($observation -join "`n"), "- screen: screen-1 .*?native=(\d+)x(\d+) origin=(-?\d+),(-?\d+)")
    if (-not $screenLine.Success) { throw "native observation did not contain screen-1 geometry" }
    $width = [int]$screenLine.Groups[1].Value
    $height = [int]$screenLine.Groups[2].Value
    $originX = [int]$screenLine.Groups[3].Value
    $originY = [int]$screenLine.Groups[4].Value
    $initial = Get-State
    $c = $initial.coordinates
    $surfaceX = To-Normalized $c.surfaceX $originX $width; $surfaceY = To-Normalized $c.surfaceY $originY $height
    $fromX = To-Normalized $c.dragFromX $originX $width; $fromY = To-Normalized $c.dragFromY $originY $height
    $toX = To-Normalized $c.dragToX $originX $width; $toY = To-Normalized $c.dragToY $originY $height
    $textX = To-Normalized $c.textX $originX $width; $textY = To-Normalized $c.textY $originY $height

    Invoke-Action "action: move_mouse`nx: $surfaceX`ny: $surfaceY"; Wait-For { (Get-State).moves -gt $initial.moves }
    Invoke-Action "action: click`nx: $surfaceX`ny: $surfaceY"; Wait-For { (Get-State).clicks -gt $initial.clicks }
    Invoke-Action "action: right_click`nx: $surfaceX`ny: $surfaceY"; Wait-For { (Get-State).rightClicks -gt $initial.rightClicks }
    Invoke-Action "action: double_click`nx: $surfaceX`ny: $surfaceY"; Wait-For { (Get-State).doubleClicks -gt $initial.doubleClicks }
    Invoke-Action "action: drag`nfrom_x: $fromX`nfrom_y: $fromY`nto_x: $toX`nto_y: $toY`nduration_ms: 200"; Wait-For { (Get-State).drags -gt $initial.drags }
    Invoke-Action "action: scroll`nx: $surfaceX`ny: $surfaceY`ndelta_y: 120"; Wait-For { (Get-State).scrollDelta -ne $initial.scrollDelta }
    Invoke-Action "action: click`nx: $textX`ny: $textY"
    $unicodeText = "Ferris $([char]0x2713)"
    Invoke-Action "action: type`ntext: $unicodeText"; Wait-For { (Get-State).text -eq $unicodeText }
    Invoke-Action "action: press_key`nkey: escape"; Wait-For { (Get-State).lastKey -eq "Escape" }
    Invoke-Action "action: click`nx: $textX`ny: $textY"
    Invoke-Action "action: hotkey`nkeys: ctrl+s"; Wait-For { (Get-State).hotkey -eq "ctrl+s" }
    $started = [DateTime]::UtcNow
    Invoke-Action "action: wait`nduration_ms: 120"
    if (([DateTime]::UtcNow - $started).TotalMilliseconds -lt 100) { throw "wait action returned too early" }
    if (-not (Get-ChildItem -Path $tracePath -Recurse -Filter screen-1.png)) { throw "native E2E produced no screenshot" }
} finally {
    if (-not $fixture.HasExited) { Stop-Process -Id $fixture.Id -Force }
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
