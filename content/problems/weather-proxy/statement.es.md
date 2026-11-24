# Proxy de una API del tiempo

## El producto

Tienes una app del tiempo. Cada vez que alguien la abre, la app pide a tu backend la previsión de su ciudad. Tú no haces previsiones: se las compras a un proveedor del tiempo externo, que cobra por llamada y solo te deja hacer unas pocas llamadas por segundo. Una previsión cambia despacio, así que una copia de hace unos minutos sirve. Lo que sí les importa a tus usuarios es una pantalla de error, así que la app debería seguir mostrando una previsión aunque el proveedor tenga un mal día.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 000 peticiones/s |
| Latencia p95 | 100 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 970 $ al mes |

## El tráfico

- **Pedir la previsión de una ciudad** (100 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos leen la previsión de una caché (cache-aside). En un fallo de caché, el App Server llama al proveedor del tiempo y guarda la respuesta en la caché. Hay 40 000 ciudades, y unas pocas grandes se llevan la mayoría de las peticiones (una popularidad Zipf de 1,0).

El proveedor del tiempo responde en unos 150 ms de mediana, con una cola lenta que llega a 600 ms en el p99. Te acepta como mucho 200 llamadas por segundo. Las llamadas por encima del límite se rechazan (un HTTP 429) y cuentan como peticiones fallidas. Un App Server conserva su hueco durante toda la llamada.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El proveedor del tiempo cae durante 40 s** y **Un nodo de caché se reinicia**. Tu diseño tiene que superar las tres.
