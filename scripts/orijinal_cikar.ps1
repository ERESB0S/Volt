<#
  Monster Pusat K3 Pro - orijinal (fabrika) yazılımını resmi güncelleme aracından çıkarır.

  Orijinal yazılım telif nedeniyle bu projeyle dağıtılmaz. Monster'ın destek sayfasındaki
  resmi sürücü paketini indir, içindeki "Upgrade Package.exe" dosyasını bu betiğe ver:

      powershell -ExecutionPolicy Bypass -File orijinal_cikar.ps1 -Exe "C:\yol\Upgrade Package.exe"

  Çıktı: ORIJINAL_K3PRO.bin (64 KB). Sonix Flasher'da "Revert to Stock Firmware" ile yüklenir.
  Betik exe'yi çalıştırmaz, sadece okur.
#>
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Cikti = "ORIJINAL_K3PRO.bin"
)
$ErrorActionPreference = "Stop"
$b = [System.IO.File]::ReadAllBytes((Resolve-Path $Exe))
$boy = 65536
$bulunan = -1
# 64 KB'lık yazılım ARM vektör tablosuyla başlar (yığın 0x20000B50) ve 16 bit kelime toplamı 0x74CC'dir.
for ($i = 0; $i -le $b.Length - $boy; $i += 4) {
    if ($b[$i] -ne 0x50 -or $b[$i + 1] -ne 0x0B -or $b[$i + 2] -ne 0x00 -or $b[$i + 3] -ne 0x20) { continue }
    $t = 0
    for ($j = $i; $j -lt $i + $boy; $j += 2) { $t = ($t + [int]$b[$j] + ([int]$b[$j + 1] -shl 8)) -band 0xFFFF }
    if ($t -eq 0x74CC) { $bulunan = $i; break }
}
if ($bulunan -lt 0) { throw "Yazılım bulunamadı. Doğru 'Upgrade Package.exe' dosyası mı? (Pusat K3 Pro, 2021-12-28)" }
$fw = New-Object byte[] $boy
[Array]::Copy($b, $bulunan, $fw, 0, $boy)
[System.IO.File]::WriteAllBytes((Join-Path (Get-Location) $Cikti), $fw)
$h = (Get-FileHash (Join-Path (Get-Location) $Cikti) -Algorithm SHA256).Hash
Write-Host "Yazıldı: $Cikti  (SHA256 $h)"
if ($h -eq "F5C413582E856DC79932D4BD818C43B37A17559456E217C9FB24F8E2D6D95066") { Write-Host "Doğrulandı: bilinen orijinal yazılım v1.00 (2021-12-28)" }
else { Write-Warning "Hash bilinen sürümle eşleşmiyor. Yüklemeden önce emin ol." }
