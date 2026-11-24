## 1

El límite del proveedor es de toda la cuenta, no de un equipo. Si un equipo envía cuarenta veces su tráfico habitual, ¿de quién son las llamadas que reciben los 429? ¿Y si la única pasarela se para, quién puede llegar al modelo?

## 2

Dale a cada equipo su propio límite antes de que sus llamadas lleguen al proveedor: un Limitador de tasa guarda un bucket por clave de API, así que la avalancha de un equipo vacía solo su propio bucket. Haz que cuente tokens, y convierte los 55 000 tokens por minuto en tokens por segundo. Comprueba que nueve equipos normales más uno en su límite se quedan por debajo de los 400 000 tokens por minuto de la cuenta. Y que la pasarela no sea una sola tarjeta.

## 3

Pon un AWS ALB delante de dos tarjetas API Gateway, LiteLLM (gateway de LLM), cada una conectada a un único Limitador de tasa compartido (Token bucket en Redis) con la Cuenta en Tokens (prompt + respuesta), un Límite de 917 tokens por segundo y una Ráfaga de 10 000 tokens. El limitador pasa las peticiones a un Servidor de aplicaciones Go que llama a una tarjeta de Inferencia de modelos (API de Claude).
