# Solución comentada

## Adónde va la carga

Con 800 URLs/s, casi todo el tiempo se va en un paso: la descarga. Tarda unos 250 ms de mediana, y un Worker conserva su hilo durante toda la espera. Solo eso supone unos 200 hilos ocupados en cada momento, más cuando aparece la cola lenta. La web admite 1 000 descargas/s, así que el margen es pequeño. Con Zipf 0,9 sobre 2 millones de URLs, una buena parte de lo que envía la frontera son repeticiones. Cada repetición que descargas ocupa un hilo y gasta límite en una página que ya guardaste. Una consulta de un milisegundo puede sustituir a una descarga de 250.

## El diseño de referencia

Un Scheduler (Airflow) publica URLs en una Queue SQS. 6 Crawler Workers con 64 hilos cada uno las toman de la Queue. Antes de descargar, cada Worker consulta una Cache Redis de URLs vistas (4 GB, TTL de un día). Un acierto significa «ya vista, sáltala», y el trabajo termina ahí. Un fallo registra la URL, descarga la página y la escribe en Cassandra con 8 particiones.

Cuesta unos 1 672 $ al mes. Su p95 ronda los 0,4 s en un rastreo normal. Con duplicados, la mayoría de las URLs se saltan y el p95 baja a unos 0,06 s. Con servidores lentos llega a unos 1,7 s.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin la Cache de URLs vistas. Supera «Un rastreo normal» con un p95 cercano a 1,3 s. Lo descarga todo, pero tiene hilos suficientes para aguantarlo.

«Un sitio lleno de enlaces duplicados» duplica el ritmo de URLs y hace las repeticiones mucho más comunes (Zipf 1,6). Sin la consulta, los Workers descargan cada repetición. Sus hilos se llenan, la Queue crece y el p95 llega a unos 38 s. «Servidores lentos» suma 1,5 s a cada descarga durante 30 s y admite un p95 de hasta 8 s. Cada descarga ocupa ahora un hilo varias veces más tiempo, el trabajo pendiente se acumula y el p95 llega a unos 23 s. La referencia aguanta ambas porque descarga muchas menos páginas, así que sus hilos quedan libres.

## Para probar

- Quita la Cache de la referencia y compara cuántas descargas llegan a la web en el rastreo normal.
- Baja el TTL a un minuto y observa cómo cae la tasa de aciertos.
- Quita Workers de uno en uno y busca el número mínimo que todavía mantiene el p95 bajo el límite cuando las descargas se vuelven lentas.
