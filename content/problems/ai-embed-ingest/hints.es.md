## 1

Tu cuenta tiene 600 000 tokens por minuto, y un día normal usa más o menos un tercio. ¿Qué le pasa a un documento que llega cuando ya se ha gastado lo del minuto, y qué le pasa a la persona que lo sube mientras la API de embeddings se toma su tiempo?

## 2

Un error de límite es un "ahora no", no un "nunca". Deja que el documento espere en un sitio duradero y vuelve a intentarlo un poco después, en vez de hacer fallar la subida. Los workers que hacen las llamadas necesitan plazas suficientes para gastar todo lo permitido, pero más plazas solo recogen más 429: el techo real es el límite del proveedor.

## 3

Haz que los App Servers (2 instancias Go) publiquen cada documento en una Cola Amazon SQS con Reintentos máximos en 10, para que una llamada rechazada vuelva tras 1 s, 2 s, 4 s y así sucesivamente. Un Worker Celery con 2 instancias y una concurrencia de 12 la consume, llaman a la tarjeta API de OpenAI (Tipo de modelo Embeddings, 3000 peticiones y 600 000 tokens por minuto) y escriben el vector en una Base de datos vectorial pgvector.
