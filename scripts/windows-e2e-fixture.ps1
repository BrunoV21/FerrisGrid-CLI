param(
    [Parameter(Mandatory = $true)][string]$StatePath,
    [Parameter(Mandatory = $true)][string]$ReadyPath
)

$ErrorActionPreference = "Stop"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class FerrisGridFixtureDpi {
    [DllImport("user32.dll")]
    private static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    public static void Enable() { try { SetProcessDpiAwarenessContext(new IntPtr(-4)); } catch { } }
}
'@
[FerrisGridFixtureDpi]::Enable()
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$state = [ordered]@{ clicks = 0; doubleClicks = 0; rightClicks = 0; moves = 0; drags = 0; scrollDelta = 0; text = ""; lastKey = ""; hotkey = ""; coordinates = @{} }
function Save-State { [System.IO.File]::WriteAllText($StatePath, ($state | ConvertTo-Json -Depth 5)) }

$form = New-Object System.Windows.Forms.Form
$form.Text = "FerrisGrid Rust Windows E2E"
$form.StartPosition = "Manual"
$form.Location = New-Object System.Drawing.Point(100, 100)
$form.ClientSize = New-Object System.Drawing.Size(640, 430)
$form.TopMost = $true
$form.KeyPreview = $true
$surface = New-Object System.Windows.Forms.Panel
$surface.Location = New-Object System.Drawing.Point(20, 20)
$surface.Size = New-Object System.Drawing.Size(600, 250)
$surface.BackColor = [System.Drawing.Color]::SteelBlue
$surface.TabStop = $true
$form.Controls.Add($surface)
$textBox = New-Object System.Windows.Forms.TextBox
$textBox.Location = New-Object System.Drawing.Point(20, 300)
$textBox.Size = New-Object System.Drawing.Size(600, 30)
$form.Controls.Add($textBox)

$surface.Add_MouseClick({ if ($_.Button -eq "Left") { $state.clicks++ }; if ($_.Button -eq "Right") { $state.rightClicks++ }; Save-State })
$surface.Add_MouseDoubleClick({ $state.doubleClicks++; Save-State })
$surface.Add_MouseMove({ $state.moves++; Save-State })
$surface.Add_MouseDown({ if ($_.Button -eq "Left") { $script:dragging = $true } })
$surface.Add_MouseUp({ if ($script:dragging) { $state.drags++ }; $script:dragging = $false; Save-State })
$surface.Add_MouseWheel({ $state.scrollDelta += $_.Delta; Save-State })
$textBox.Add_TextChanged({ $state.text = $textBox.Text; Save-State })
$form.Add_KeyDown({ $state.lastKey = $_.KeyCode.ToString(); if ($_.Control -and $_.KeyCode -eq "S") { $state.hotkey = "ctrl+s" }; Save-State })
$form.Add_Shown({
    $surface.Focus()
    $center = $surface.PointToScreen((New-Object System.Drawing.Point(300, 125)))
    $from = $surface.PointToScreen((New-Object System.Drawing.Point(80, 80)))
    $to = $surface.PointToScreen((New-Object System.Drawing.Point(500, 180)))
    $text = $textBox.PointToScreen((New-Object System.Drawing.Point(300, 15)))
    $state.coordinates = [ordered]@{ surfaceX = $center.X; surfaceY = $center.Y; dragFromX = $from.X; dragFromY = $from.Y; dragToX = $to.X; dragToY = $to.Y; textX = $text.X; textY = $text.Y }
    Save-State
    [System.IO.File]::WriteAllText($ReadyPath, "ready")
})
[System.Windows.Forms.Application]::Run($form)
