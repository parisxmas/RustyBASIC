' NTP clock example
WIFI.CONNECT "MySSID", "MyPassword"
DELAY 3000
NTP.SYNC "pool.ntp.org"
NTP.TIME$ timeStr$
PRINT "Current time: "; timeStr$
NTP.EPOCH epoch%
PRINT "Unix epoch: "; epoch%
