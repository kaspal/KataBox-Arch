# Asistente de voz

## El producto

Llevas el asistente de voz de una app de cocina. La gente le habla con las manos ocupadas: «¿y ahora qué?», «¿cuánto tarda la cebolla?». Mientras la app está en pantalla, mantiene abierta una sesión WebSocket con tu backend. Cuando el usuario deja de hablar, la app envía el audio por esa sesión, y tu backend responde en tres pasos: un modelo de voz a texto escribe lo que se ha dicho, un modelo de lenguaje escribe una respuesta corta y un modelo de texto a voz convierte la respuesta en audio, que suena en el teléfono. En una conversación, una pausa de más de un segundo o así parece una avería, así que lo que importa es el tiempo desde el final de las palabras del usuario hasta el primer sonido de la respuesta.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 turnos/s |
| Latencia p95 (hasta el primer sonido de la respuesta) | 1200 ms o menos |
| Sesiones conectadas | 95 % o más, en todo momento |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 10 000 $ al mes |

## El tráfico

- **Dispositivos con una sesión abierta**. 150 000 teléfonos mantienen una sesión WebSocket abierta en tus tarjetas de Gateway de WebSocket. Casi todo el tiempo están en silencio.
- **Un turno hablado** (el 100 % de los turnos). El turno viaja por la sesión hasta un Gateway de WebSocket, que lo pasa a tus Servidores de aplicaciones. El Servidor de aplicaciones llama a tres tarjetas de Inferencia de modelos en orden, y conserva su hueco hasta que responde la tercera:
  1. **Voz a texto.** Pon el Tipo de modelo de esta tarjeta en «Embeddings»: en el simulador es un modelo que lee toda su entrada y no escribe nada que cuente como salida. Su tiempo es su Tiempo hasta el primer token más la entrada dividida entre su Velocidad del prompt.
  2. **El modelo de lenguaje.** Tipo de modelo «Chat (generación de texto)». Lee unos 300 tokens (el paso de la receta, la conversación hasta ahora y la transcripción; 450 en el p95) y escribe una respuesta de unos 15 tokens (30 en el p95): una respuesta hablada es una frase corta. Su tiempo es su Tiempo hasta el primer token, más el prompt dividido entre su Velocidad del prompt, más la respuesta dividida entre su Velocidad de salida.
  3. **Texto a voz.** Tipo de modelo «Clasificador»: en el simulador escribe una unidad de salida, que representa el primer trozo de audio. El usuario oye la respuesta desde ese momento. Pon su Velocidad del prompt en 50 000 tokens/s, porque solo lee la respuesta, no el prompt entero.

El simulador saca un tamaño de prompt por turno y se lo cobra a las tres llamadas a modelos; en la tarjeta de voz a texto representa la duración del audio.

Puedes alojar los modelos tú mismo (vLLM en GPU para el modelo de lenguaje, NVIDIA Triton para los modelos de voz) o llamar a una API de modelos alojada. Deja el Tiempo hasta el primer token, la Velocidad de salida y la Velocidad del prompt de cada tarjeta en los valores por defecto de la tarjeta, salvo la Velocidad del prompt de la tarjeta de texto a voz de arriba: representan el modelo que eliges, no un mando que puedas girar. En una GPU propia, una tarjeta vLLM ejecuta 16 peticiones a la vez por GPU (Huecos de lote por GPU), y en el simulador cada petición escribe sus tokens más despacio cuantos más huecos están ocupados: a la Velocidad de salida × (1 − 0,5 × huecos ocupados ÷ todos los huecos). Una API alojada no tiene huecos que llenar y cobra cada token, y su cuenta permite 4000 peticiones y 400 000 tokens por minuto por tarjeta, salvo que subas los límites.

La app abandona un turno a los 5 s (el Tiempo límite por defecto del Cliente).

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Un modelo va 300 ms más lento** y **El triple de turnos durante 30 s**. Tu diseño tiene que pasar las tres.
