' Cron scheduling example
CRON.ADD 1, "*/5"
PRINT "Cron job added (every 5 minutes)"
CRON.CHECK 1, fired%
PRINT "Should fire: "; fired%
CRON.REMOVE 1
PRINT "Cron job removed"
