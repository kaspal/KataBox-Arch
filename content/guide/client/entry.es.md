Un cliente es quien envía peticiones a tu sistema: un navegador, una app móvil o el servidor de otra empresa que llama a tu API. En un diseño es donde empieza cada petición, y nunca hace trabajo propio.

En el simulador, las peticiones salen del cliente en momentos aleatorios, como usuarios reales, y cada una sigue los cables hasta un componente que pueda servirla. Si no llega respuesta en unos segundos, el cliente se rinde y la petición cuenta como error.

En el Sandbox los **rps** del cliente fijan la carga. En un problema la carga total la fija el escenario, y varias tarjetas de cliente se la reparten en proporción a sus rps.
