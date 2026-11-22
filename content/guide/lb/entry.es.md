Un balanceador de carga da a muchos servidores una sola puerta de entrada. Los clientes se conectan al balanceador, y este reenvía cada petición a uno de los servidores que tiene detrás, según un **algoritmo**, como ir por turnos o elegir el menos ocupado. Para crecer, añades servidores detrás; los clientes ni se enteran.

También hace **comprobaciones de salud**: pregunta a cada servidor, cada pocos segundos, si sigue vivo. Un servidor que falla un par de comprobaciones seguidas sale de la rotación, y uno que se recupera vuelve a entrar. Hasta que se detecta un fallo, las peticiones enviadas al servidor caído fallan.

En el simulador el balanceador añade un breve tiempo de reenvío y tiene capacidad de sobra. Su trabajo real es repartir la carga y esquivar las tarjetas caídas, así que son los servidores de detrás los que deciden cuánto tráfico aguanta el diseño.
