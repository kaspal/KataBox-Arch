# Códigos promocionales

## El producto

Tienes el servicio de códigos promocionales de una tienda online. Al pagar, el cliente escribe un código y la tienda pregunta a tu servicio si es válido y cuántos usos le quedan. Algunos códigos solo se pueden usar un número fijo de veces: «los primeros 500 pedidos tienen un 20 % de descuento». Cuando el cliente hace el pedido, la tienda canjea el código y tu servicio le resta un uso. Un código que se usa más veces de las que permite le cuesta dinero de verdad a la tienda, así que un código limitado nunca debe canjearse de más. Además, el pago tiene que seguir funcionando cuando la base de datos tiene un mal día.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 rps |
| Latencia p95 | 100 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 1 % o menos |
| Presupuesto | 1300 $ al mes |

## El tráfico

Las peticiones de los clientes llegan a tus App Servers, a través de un edge si quieres uno.

- **Comprobar un código al pagar** (95 %). Un App Server busca el código en el almacén de datos, a través de una caché si añades una. Hay 5000 códigos, y unos pocos populares se llevan la mayoría de las consultas.
- **Canjear un código de usos limitados** (5 %). Un App Server lee del almacén de datos los usos que le quedan al código y después escribe la nueva cuenta. El 30 % de los canjes son de un único código que ahora mismo usa todo el mundo. Si dos canjes leen «queda 1 uso» antes de que ninguno escriba, los dos salen bien, y el código se usa una vez más de lo que permite. El simulador lo cuenta como un conflicto.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una influencer comparte un código** y **Falla la primaria de la base de datos**. Tu diseño tiene que superar las tres.
