# Ingesta de embeddings

## El producto

Llevas la búsqueda de un producto de centro de ayuda. Tus clientes añaden sus propios documentos (artículos, manuales, políticas), y sus usuarios luego los buscan por significado, no solo por palabras exactas. Para que un documento se pueda buscar, tu backend envía su texto a una API de embeddings alojada, recibe un vector y lo guarda en una base de datos vectorial. Tú no ejecutas el modelo: lo pagas por token, y tu cuenta solo puede enviar cierto número de peticiones y de tokens por minuto. Nadie espera delante de una pantalla por un documento, pero un documento debería aparecer en la búsqueda poco después de añadirse, y nunca debe perderse sin que nadie se entere.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5 documentos/s |
| Latencia p95 hasta completar | 30 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Presupuesto | 35 000 $ al mes |
| Mensajes perdidos | Ninguno |
| Mensajes en la Cola de mensajes fallidos | Ninguno |

## El tráfico

Hay un tipo de petición.

- **Añadir un documento** (100 %). La subida llega a tus App Servers, quizá a través de un componente de borde. Después, el texto del documento va a una tarjeta de Inferencia de modelos con el Tipo de modelo en Embeddings, y el vector que devuelve se escribe en una Base de datos vectorial. El App Server puede hacer esas llamadas él mismo, o publicar el documento en un broker para que un Worker las haga.

Un documento que acaba en una Cola de mensajes fallidos se guarda, pero nunca llega a la búsqueda, así que el juez lo cuenta: no se permite ninguno.

La latencia se mide hasta completar: desde que llega la subida hasta que el vector queda guardado. Un documento típico tiene unos 500 tokens; uno de cada veinte pasa de 1500 tokens. Por esos documentos largos, la media es de unos 625 tokens.

La API de embeddings es el sabor API de OpenAI. Deja su Tiempo hasta el primer token (500 ms) y su Velocidad del prompt (5000 tokens por segundo) como están: describen al proveedor, no tu diseño. Los límites de tu cuenta son **3000 peticiones por minuto** y **600 000 tokens por minuto**. No son una decisión de diseño: el simulador los aplica a cada tarjeta de API de OpenAI (el Inspector los muestra, sin poder cambiarlos). Una llamada por encima de un límite se rechaza con un error de límite (un HTTP 429). Usa el Modo de API Tiempo real: la API por lotes devuelve sus resultados horas después, muy por encima del objetivo de 30 s.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un cliente importa su archivo (×10 durante 45 s)** y **La API de embeddings se vuelve lenta**. Tu diseño tiene que superar las tres.
