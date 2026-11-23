## 1

El producto solo necesita saber que su evento está a salvo. ¿Tiene que esperar también a que se pueda buscar? ¿Y qué pasa con un evento que llega mientras el clúster de búsqueda no puede aceptarlo?

## 2

Pon un stream entre los App Servers y el índice. El App Server confirma el evento en cuanto está en el stream, y unos workers lo indexan desde ahí al ritmo que el índice aguante. Así, una ráfaga espera en el stream, y una caída solo retrasa la indexación: el stream guarda cada registro hasta que un worker lo ha escrito.

## 3

Conecta unos App Servers Go para que publiquen en un Stream Apache Kafka, y unos indexadores Celery para que lo consuman y escriban en OpenSearch; las búsquedas siguen yendo directamente de los App Servers a OpenSearch. Un stream entrega cada partición a un solo consumidor a la vez, así que sube Particiones muy por encima del valor por defecto (la referencia usa 128) o los indexadores se quedan parados. 6 instancias Celery con una Concurrencia por instancia de 32, 2 App Servers y 6 Shards de OpenSearch bastan de sobra.
