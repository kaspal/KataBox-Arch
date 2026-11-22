## 1

¿Con qué frecuencia descarga tu rastreador una página que ya tiene, y cuánto cuesta cada una de esas descargas?

## 2

Elimina duplicados antes del paso caro. Una consulta rápida que dice «ya vista» cuesta mucho menos que un viaje a la web, y ahorra un hilo de Worker y parte del límite de la web.

## 3

Pon una Cache Redis de URLs vistas entre el Worker y la web, consultada antes de cada descarga, con un TTL largo, por ejemplo un día. Da a los Workers suficientes hilos en total para seguir avanzando cuando las páginas tardan segundos en lugar de milisegundos.
