' HTTPS client example
WIFI.CONNECT "MySSID", "MyPassword"
DELAY 3000
HTTPS.GET$ "https://httpbin.org/get", response$
PRINT "GET response: "; response$
DIM body$ AS STRING
body$ = "{key: value}"
HTTPS.POST$ "https://httpbin.org/post", body$, result$
PRINT "POST response: "; result$
