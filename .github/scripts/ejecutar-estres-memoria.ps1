$ErrorActionPreference = "Stop"

$ejecutable = Get-ChildItem "target/release/deps/memoria-*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if ($null -eq $ejecutable) {
    throw "No se encontró el ejecutable de pruebas de memoria"
}

$salida = "estres-windows.stdout.log"
$errores = "estres-windows.stderr.log"
$argumentos = @(
    "--ignored",
    "--nocapture",
    "estres_prolongado_deberia_estabilizar_modulos_jit_y_memoria"
)
$proceso = Start-Process -FilePath $ejecutable.FullName `
    -ArgumentList $argumentos `
    -RedirectStandardOutput $salida `
    -RedirectStandardError $errores `
    -PassThru

$inicial = $null
$maximo = 0L
while (-not $proceso.HasExited) {
    Start-Sleep -Milliseconds 200
    $proceso.Refresh()
    $rss = $proceso.WorkingSet64
    if ($null -eq $inicial) {
        $inicial = $rss
    }
    $maximo = [Math]::Max($maximo, $rss)
}
$proceso.WaitForExit()
$proceso.Refresh()
$final = $proceso.WorkingSet64
if ($null -eq $inicial) {
    $inicial = $final
}

Get-Content $salida
if ($proceso.ExitCode -ne 0) {
    Get-Content $errores
    throw "La campaña terminó con código $($proceso.ExitCode)"
}
$limite = $inicial + 64 * 1024 * 1024
Write-Output "estres_windows rss_inicial=$inicial rss_final=$final rss_maximo=$maximo limite=$limite"
if ($final -gt $limite) {
    throw "RSS creció de $inicial a $final tras estabilizar"
}
