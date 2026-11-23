## 1

Cada fallo de caché hace una búsqueda vectorial, y una búsqueda ocupa uno de los 16 huecos del shard. ¿Cuántas búsquedas por segundo puede terminar un shard, y cuántas de ellas usa un día normal? Después piensa en el comprador que abre el mismo artículo que miles de personas más: ¿a qué shard va esa búsqueda, y cambia algo si añades shards?

## 2

Una búsqueda sobre 5 millones de vectores en un solo shard tarda unos 15 ms, así que un shard termina como mucho unas 1000 búsquedas por segundo, menos si cuentas la cola lenta. Un día normal ya lo tiene ocupado unos dos tercios del tiempo. Más shards añaden huecos, pero las búsquedas de un artículo caen siempre en el mismo shard, así que un único artículo muy popular puede desbordar su shard por muchos que haya. La lista terminada es la misma para todos los que abren ese artículo: una caché puede responder esas peticiones antes de que lleguen a la búsqueda.

## 3

Pon un AWS ALB delante de 4 App Servers en Go. Añade una Caché Redis con 4 GB de memoria y un TTL de 300 s, consultada desde los App Servers. Busca en una Base de datos vectorial Qdrant con 2 shards y 5 millones de vectores. Reordena en NVIDIA Triton, 1 instancia, tipo de modelo «Clasificador».
