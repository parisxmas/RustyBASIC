' Struct (TYPE) example — QBASIC style OOP

TYPE Point
    x AS SINGLE
    y AS SINGLE
END TYPE

TYPE Rect
    left AS SINGLE
    top AS SINGLE
    width AS SINGLE
    height AS SINGLE
END TYPE

DECLARE FUNCTION RectArea! (r AS Rect)
DECLARE SUB PrintPoint (p AS Point)

DIM origin AS Point
origin.x = 0.0
origin.y = 0.0

DIM cursor AS Point
cursor.x = 10.5
cursor.y = 20.3

CALL PrintPoint(origin)
CALL PrintPoint(cursor)

DIM r AS Rect
r.left = 0
r.top = 0
r.width = 100
r.height = 50

PRINT "Rectangle area:"; RectArea!(r)

END

SUB PrintPoint (p AS Point)
    PRINT "Point("; p.x; ","; p.y; ")"
END SUB

FUNCTION RectArea! (r AS Rect)
    RectArea! = r.width * r.height
END FUNCTION
