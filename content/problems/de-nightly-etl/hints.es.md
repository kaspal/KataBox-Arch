## 1

Una base de datos atiende a dos lectores muy distintos: la app, que lee un pedido pequeño cada vez y necesita una respuesta rápida, y la exportación, que lee páginas enormes tan rápido como puede. Calcula cuántas conexiones de la base de datos mantiene ocupadas cada uno. ¿Qué les pasa a las lecturas de la app cuando la parte de la exportación se triplica?

## 2

La exportación solo lee, y no necesita la última escritura. Una réplica de lectura tiene los mismos datos que la principal, así que puede quitarle esas lecturas a la principal. Las escrituras siguen teniendo que ir a la principal. Mira qué hace con las lecturas el ajuste «Leer de» de una Base de datos SQL cuando tiene réplicas.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go conectados por Consulta a una Base de datos SQL PostgreSQL (la principal). Un Planificador Airflow publica en una Cola RabbitMQ, y un Worker Celery (4 instancias, concurrencia por instancia 64, tiempo de servicio 2 ms) la consume y se conecta por Consulta a una segunda Base de datos SQL PostgreSQL. Conecta la principal con esa segunda mediante Replicar, y deja «Leer de» en «Principal y réplicas».
