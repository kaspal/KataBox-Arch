Una cola de mensajes fallidos, o **DLQ**, recoge los mensajes que una cola o un stream no pudieron entregar. Cuando un mensaje ha fallado todos los reintentos, el broker lo mueve a la DLQ en lugar de descartarlo. Ahí nada se procesa solo: espera a que una persona o una herramienta lo revise, arregle la causa y lo reprocese.

Protege frente a dos problemas. Una caída de un proveedor que dura más que los reintentos tiraría todos los mensajes enviados durante ella. Y un **mensaje envenenado**, uno que nunca puede tener éxito, se reintentaría para siempre y haría perder tiempo a los workers.

Una DLQ solo ayuda si alguien la vigila. Pon una alerta cuando caigan mensajes en ella, y ten una forma sencilla de devolverlos a la cola principal una vez arreglada la causa. La retención es cuánto tiempo esperan antes de caducar.

**Cómo lo factura el simulador.** Una cola de mensajes fallidos se cobra como una cola SQS: 0,40 $ por millón de peticiones, tres por mensaje (enviar, recibir y borrar), así que 1,20 $ por millón de mensajes que caen en ella. No hay coste por cola, así que una DLQ que sigue vacía no cuesta nada (precio de lista, us-east-1, consultado el 2026-11-22).
