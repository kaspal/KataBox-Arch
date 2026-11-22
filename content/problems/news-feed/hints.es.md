## 1

Cuando alguien abre su feed, ¿dónde debería estar ya esperándolo, para que la petición no tenga nada que montar?

## 2

Usa fan-out en escritura: cuando se publica un post, cópialo por adelantado en el feed de cada seguidor. Haz esa copia en Workers, fuera del camino de la petición, y guarda los feeds terminados en una caché para que las lecturas casi nunca lleguen a la base de datos.

## 3

Publica los posts en un Stream con bastantes particiones y deja que unos pocos Workers de fan-out lo consuman y escriban en una Cache Redis de feeds grande (16 GB van sobrados). Lee los feeds de esa Cache con cache-aside, dale una réplica a PostgreSQL y deja a los App Servers mucho margen por encima de un día normal.
