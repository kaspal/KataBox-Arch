# Geolocalización por IP

## El producto

Tienes un servicio de consulta que dice a otras aplicaciones dónde está una dirección IP: el país, la región y la ciudad. Lo llaman servidores de anuncios, controles de fraude, tiendas que eligen la moneda y sitios que eligen el idioma, a menudo en cada página que sirven. La respuesta sale de una tabla de alrededor de 1 millón de rangos de IP. Nadie escribe en ella mientras el servicio funciona; una copia nueva llega una vez por semana. Quien llama espera tu respuesta antes de poder hacer nada más, así que tiene que ser rápida, y tiene que seguir respondiendo cuando una parte de la nube desaparece.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 30 000 peticiones/s |
| Latencia p95 | 40 ms o menos |
| Errores | 0,1 % o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1800 $ al mes |

## El tráfico

Quien llama llega al servicio por un borde opcional, como un DNS y un Load Balancer, después a un App Server y después a una Cache. Aquí la Cache no es un atajo delante de una base de datos: guarda la tabla de IP entera y es el único sitio donde viven los datos. Hay 1 millón de rangos de IP, y su popularidad sigue una curva Zipf 0,8: unas direcciones se consultan mucho más que otras.

- **Consultar una dirección IP** (100 %). El App Server lee de la Cache el rango de la IP y devuelve su ubicación.

Puedes colocar tarjetas en regiones. Quien llama está fuera de todas las regiones.

No interviene ningún proveedor externo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **La hora más ocupada de la semana** y **Se apaga una región**. Tu diseño tiene que superar las tres.
