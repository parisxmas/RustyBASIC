' NVS (Non-Volatile Storage) example for ESP32-C3 (QBASIC style)
' Stores and retrieves a boot counter

DIM counter AS INTEGER

NVS.READ "boots", counter
counter = counter + 1
NVS.WRITE "boots", counter

PRINT "Boot count: "; counter

END
