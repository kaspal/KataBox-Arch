## 1

Dos Zonas de disponibilidad te protegen de que falle un centro de datos. ¿Qué te protege cuando fallan a la vez todas las zonas de una región, o cuando la región está bien pero aislada de tus clientes?

## 2

Haz funcionar el servicio en más de una Región, cada una con su propio Load Balancer, sus App Servers y una copia de la base de datos, y deja que DNS reparta a los clientes entre ellas. Las lecturas pueden quedarse en su propia Región; las escrituras siguen teniendo que llegar al primario, así que comprueba cuánto cuesta una escritura entre regiones frente al objetivo de p95.

## 3

Pon Route 53 delante de tres Regiones. En cada una, coloca un AWS ALB, un App Server Go con 2 instancias y un miembro PostgreSQL: el primario en una Región y una réplica en cada una de las otras dos, conectadas con enlaces Replicate. Cada App Server habla con la base de datos de su propia Región. Una escritura desde otra Región cruza dos veces, unos 150 ms en total, que siguen cabiendo en 250 ms. Cuando se pierde la Región del primario, una réplica pasa a primario.
