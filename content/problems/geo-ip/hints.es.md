## 1

La tabla de IP entera es pequeña, y nadie escribe en ella mientras el servicio funciona. ¿Qué abarata eso que sería difícil con datos que la gente cambia? ¿Y qué les pasa a quienes te llaman si desaparece el único sitio que la guarda?

## 2

Pon una copia completa de la tabla en memoria en dos regiones, cada una con su propio Load Balancer y sus App Servers, y deja que un DNS con comprobaciones de salud mande a quien llama a las regiones que responden. Cuando se pierde una región, la otra recibe a todos, así que dimensiona cada región para llevar sola todo el pico, y encima la hora más ocupada.

## 3

Pon Route 53 con un TTL de 30 s delante de dos regiones, us-east y eu-west. En cada una, coloca un AWS ALB, Go App Servers con 6 instancias y una Redis Cache con 2 GB de memoria y un TTL de 0, para que los datos no caduquen nunca. Cada App Server lee de la Cache de su propia región. Con 6 instancias, las aplicaciones de cada región van ocupadas menos de un tercio en un día normal, así que cualquiera de las dos regiones puede llevar todo el tráfico cuando la otra se apaga.
