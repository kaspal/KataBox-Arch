Las métricas son números que se miden una y otra vez a lo largo del tiempo: peticiones por segundo, tasa de errores, percentiles de latencia, CPU, profundidad de cola. Cada servicio expone sus números, un recolector como Prometheus los recoge cada pocos segundos, y una herramienta como Grafana los dibuja en gráficas y lanza alertas cuando una línea cruza un umbral.

Son el primer sitio donde mirar cuando algo va mal. Una métrica te dice *que* la latencia ha subido y más o menos dónde; los logs y las trazas te dicen después *por qué*. Como solo guardan números, las métricas siguen siendo baratas incluso con muchísimo tráfico.

Las métricas nunca están en la ruta de una petición, así que en el simulador no añaden latencia ni pueden descartar una petición. Solo suman su coste mensual, y un problema puede exigirlas como parte de un diseño listo para producción.
