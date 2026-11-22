## 1

Cuando un cliente se porta mal, ¿quién debería pagarlo: ese cliente o todos los que comparten tus App Servers?

## 2

Dale a cada clave de API su propia cubeta de tokens en el borde, para que una clave que se queda sin tokens nunca afecte a las demás.

## 3

Pon un Rate Limiter delante de los App Servers en lugar de un Load Balancer simple. Fija el límite por clave muy por encima del momento más intenso de un cliente real, al menos el doble de sus 59 peticiones/s normales, y añade una ráfaga encima.
