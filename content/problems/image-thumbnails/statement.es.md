# Miniaturas de imágenes

## El producto

Llevas el backend de fotos de un grupo de apps móviles. La gente hace una foto, pulsa subir y espera verla en su galería como una pequeña vista previa un momento después. Cada subida es una foto completa de unos 2 MB. Guardar el original es rápido. Hacer las miniaturas no lo es: un servidor tiene que decodificar la imagen entera y redimensionarla, y eso lleva tiempo de verdad. La subida no debería quedarse esperando ese trabajo, pero el trabajo tiene que hacerse siempre. Una foto que nunca recibe sus miniaturas es una foto rota.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 300 subidas/s |
| Latencia p95 hasta completar | 5 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 14 500 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Hay un solo tipo de petición.

- **Subir una foto** (100 %). La subida puede pasar por un componente de borde, como un Load Balancer, y llega a la Upload API, un App Server. Esta escribe el original de 2 MB en el almacenamiento de objetos y publica un trabajo de miniatura en un broker. Un Worker toma el trabajo, decodifica y redimensiona la foto, y vuelve a escribir las miniaturas en el almacenamiento de objetos.

La latencia se mide hasta completar: desde que llega la subida hasta que las miniaturas quedan guardadas, no solo hasta que responde la Upload API. Decodificar y redimensionar una foto de 2 MB le lleva a un Worker unos 200 ms, y conserva su hueco durante todo el trabajo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una avalancha de subidas** y **Se cae un worker**. Tu diseño tiene que superar las tres.
