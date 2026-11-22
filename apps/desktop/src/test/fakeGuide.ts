// Two Guide entries for UI tests (the shapes Rust sends: ka-app guide.rs).
import type { GuideEntry } from "@/bindings";

export const FAKE_CACHE: GuideEntry = {
  kind: "cache",
  one_liner: "A fast, in-memory copy of the data you read most.",
  one_liner_es: "Una copia rápida, en memoria, de los datos que más lees.",
  use_when: ["Reads far outnumber writes", "The same keys are read again and again", "Data a few seconds old is acceptable", "The database is your bottleneck"],
  use_when_es: ["Las lecturas superan a las escrituras", "Las mismas claves se leen una y otra vez", "Se aceptan datos de hace unos segundos", "La base de datos es tu cuello de botella"],
  skip_when: ["Every read must see the latest write", "Writes dominate"],
  skip_when_es: ["Cada lectura debe ver la última escritura", "Dominan las escrituras"],
  sim: [
    { prop: "memory_gb", note: "How much of the hot set fits.", note_es: "Cuánto cabe del conjunto caliente." },
    { sim: "servers_per_instance", note: "Parallel work per node.", note_es: "Trabajo en paralelo por nodo." },
    { cost: "per_gb_month", note: "Priced by memory.", note_es: "Se cobra por memoria." },
  ],
  formula: "hit rate ≈ share of reads in the hot set that fits",
  formula_es: "tasa de aciertos ≈ lecturas del conjunto caliente que cabe",
  flavours_compared: {
    flavours: ["redis", "memcached"],
    rows: [{ label: "Threads", label_es: "Hilos", cells: ["One thread", "Multi-threaded"], cells_es: ["Un hilo", "Multihilo"] }],
  },
  pitfalls: [
    { title: "Cache stampede", title_es: "Estampida", body: "A popular key expires and every miss hits the database.", body_es: "Caduca una clave popular.", fix: "Coalesce requests.", fix_es: "Agrupa peticiones.", seen_in: "url-shortener" },
  ],
  gains: ["Fast reads"],
  gains_es: ["Lecturas rápidas"],
  costs: ["Stale reads"],
  costs_es: ["Lecturas obsoletas"],
  works_with: ["app", "sql"],
  practice: ["url-shortener"],
  interview_tip: "Say what you cache and for how long.",
  interview_tip_es: "Di qué cacheas y cuánto tiempo.",
  sources: [
    { flavour: "redis", title: "Redis documentation", url: "https://redis.io/docs/latest/", checked: false },
    { flavour: "memcached", title: "Memcached wiki", url: "https://github.com/memcached/memcached/wiki", checked: false },
  ],
};

export const FAKE_LB: GuideEntry = {
  ...FAKE_CACHE,
  kind: "lb",
  one_liner: "Spreads traffic over servers.",
  one_liner_es: "Reparte el tráfico entre servidores.",
  sim: [{ prop: "algorithm", note: "How the next server is chosen.", note_es: "Cómo se elige el siguiente servidor." }],
  flavours_compared: null,
  practice: [],
  sources: [],
};

export const FAKE_GUIDE = [FAKE_LB, FAKE_CACHE];
