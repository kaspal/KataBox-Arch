## 1

Un turno espera a tres modelos, uno detrás de otro, así que el usuario oye la suma de sus tiempos. Súmalos para un turno típico y para uno largo. ¿Cuánto sobra de los 1,2 s? ¿Y qué pasa con ese tiempo de sobra cuando una etapa va más lenta, o cuando habla el triple de gente a la vez?

## 2

Elige modelos con un tiempo fijo de arranque pequeño: el primer token de las API alojadas tarda de 500 a 600 ms, la mitad del presupuesto antes de escribir una sola palabra. Un modelo de lenguaje alojado por ti arranca antes, pero en una GPU cada hueco ocupado frena a los demás, y una GPU llena pone los turnos en cola. Dimensiona las GPU para que sigan casi libres en el pico y con el triple del pico; ese tiempo libre es el margen que necesita tu presupuesto del turno.

## 3

Un Cliente móvil se conecta por un enlace WebSocket a un Gateway de WebSocket Socket.IO (2 instancias), que pasa los turnos a 2 Servidores de aplicaciones Go. Los Servidores de aplicaciones llaman a tres tarjetas de Inferencia de modelos: un NVIDIA Triton con el Tipo de modelo «Embeddings» para la voz a texto, un vLLM en GPU con 4 instancias para el modelo de lenguaje, y un NVIDIA Triton con el Tipo de modelo «Clasificador» y una Velocidad del prompt de 50 000 tokens/s para el texto a voz.
