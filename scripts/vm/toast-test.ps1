# Shows one test toast under -AppId, from the desktop session (Phase 5).
param([string]$AppId, [string]$Text)
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$x = New-Object Windows.Data.Xml.Dom.XmlDocument
$x.LoadXml("<toast><visual><binding template=`"ToastGeneric`"><text>XkyHold-wd test</text><text>$Text</text></binding></visual></toast>")
try { [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($AppId).Show([Windows.UI.Notifications.ToastNotification]::new($x)); "shown $AppId" } catch { "ERR $_" }
