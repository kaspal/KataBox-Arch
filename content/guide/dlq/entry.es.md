Una cola de mensajes fallidos, o **DLQ**, recoge los mensajes que una cola o un stream no pudieron entregar. Cuando un mensaje ha fallado todos los reintentos, el broker lo mueve a la DLQ en lugar de descartarlo. Ahí nada se procesa solo: espera a que una persona o una herramienta lo revise, arregle la causa y lo reprocese.

Protege frente a dos problemas. Una caída de un proveedor que dura más que los reintentos tiraría todos los mensajes enviados durante ella. Y un **mensaje envenenado**, uno que nunca puede tener éxito, se reintentaría para siempre y haría perder tiempo a los workers.

Una DLQ solo ayuda si alguien la vigila. Pon una alerta cuando caigan mensajes en ella, y ten una forma sencilla de devolverlos a la cola principal una vez arreglada la causa. La retención es cuánto tiempo esperan antes de caducar.
