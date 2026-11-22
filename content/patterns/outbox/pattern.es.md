En lugar de escribir en la base de datos y después publicar en un broker, el servicio escribe el cambio y una fila outbox que describe el evento en una sola transacción local. O se guardan los dos o ninguno, así que nunca pueden discrepar.

Un proceso de reenvío aparte lee las filas outbox nuevas y las publica en el stream. Puede consultar la tabla periódicamente o seguir el registro de cambios de la base de datos (change data capture). Cuando el broker acepta un evento, el proceso marca la fila como enviada.

Si el proceso cae después de publicar pero antes de marcar la fila, vuelve a publicar el evento al arrancar. Por eso los consumidores deben tolerar duplicados, normalmente recordando los ids que ya han procesado.
