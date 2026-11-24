# Sobrevivir a la caída de una región

## El producto

Gestionas el servicio de cuentas de una app que se usa en todo el mundo. La gente abre la página de su cuenta a todas horas y, de vez en cuando, cambia algo: un nombre, una dirección, un ajuste. El resto de la app le pregunta a este servicio quién es el usuario, así que, cuando se cae, se cae la app entera. Que falle una máquina es lo normal. Esta vez la pregunta es mayor: ¿qué pasa cuando se apaga una región entera de la nube, o sigue en pie pero ya no puede hablar con el resto del mundo?

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 6000 peticiones/s |
| Lecturas | 90 % de las peticiones |
| Latencia p95 | 250 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1600 $ al mes |

## El tráfico

Los clientes llegan al servicio a través de un borde opcional, como DNS y un Load Balancer, después un App Server, una Cache opcional y una base de datos. Hay 1 millón de cuentas, y su popularidad sigue una curva Zipf 1,0: algunas cuentas se leen y se cambian muchísimo más que otras.

- **Leer una cuenta** (90 %). El App Server lee la cuenta de la base de datos, o de una Cache si añades una.
- **Actualizar una cuenta** (10 %). El App Server escribe la cuenta en el primario de la base de datos.

Puedes colocar tarjetas en Regiones, y en Zonas de disponibilidad dentro de ellas. Una llamada entre dos Regiones tarda 70 ms en cada sentido. Una lectura va a un miembro de la base de datos en la propia Región de quien llama, si lo hay; una escritura va siempre al primario, esté donde esté.

No intervienen proveedores externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se apaga la región del primario** y **La región del primario queda aislada 20 s**. Tu diseño tiene que superar las tres.
