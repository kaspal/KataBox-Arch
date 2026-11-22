// Two Guide entries for UI tests (the shapes Rust sends: ka-app guide.rs).
import type { DecideFlow, Design, GlossaryTerm, GuideEntry, NumberFigure, PatternEntry } from "@/bindings";

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

export const FAKE_FLOW: DecideFlow = {
  id: "sql-or-nosql",
  number: 1,
  title: "SQL or NoSQL?",
  title_es: "¿SQL o NoSQL?",
  intro: "Two questions decide it.",
  intro_es: "Dos preguntas lo deciden.",
  start: "joins",
  questions: [
    {
      id: "joins",
      text: "Do you need joins across your data?",
      text_es: "¿Necesitas joins entre tus datos?",
      options: [
        { label: "Yes", label_es: "Sí", result: "sql", why: "Joins are what relational databases do best.", why_es: "Los joins son lo que mejor hacen." },
        { label: "No", label_es: "No", next: "scale", why: "w", why_es: "w" },
      ],
    },
    {
      id: "scale",
      text: "Must it grow past one machine?",
      text_es: "¿Debe crecer más allá de una máquina?",
      options: [
        { label: "Yes", label_es: "Sí", result: "nosql", why: "Partitioning by key scales out.", why_es: "Particionar por clave escala." },
        { label: "No", label_es: "No", result: "sql", why: "One database is simpler.", why_es: "Una base de datos es más simple." },
      ],
    },
  ],
};

export const FAKE_TERMS: GlossaryTerm[] = [
  { id: "ttl", term: "TTL", term_es: "TTL", definition: "How long a cached copy lives.", definition_es: "Cuánto vive una copia en caché.", see: "cache" },
  { id: "failover", term: "Failover", term_es: "Failover", definition: "A replica takes over from a dead primary.", definition_es: "Una réplica sustituye a una principal caída.", see: null },
];

export const FAKE_PATTERN: PatternEntry = {
  id: "cache-aside",
  number: 1,
  title: "Cache-aside",
  title_es: "Cache-aside",
  one_liner: "Read the cache first; on a miss, read the database and fill the cache.",
  one_liner_es: "Lee primero la caché; en un fallo, lee la base de datos y llena la caché.",
  problem: "The database can't serve every read.",
  problem_es: "La base de datos no puede servir todas las lecturas.",
  use_when: ["Reads dominate"],
  use_when_es: ["Dominan las lecturas"],
  gains: ["Fast reads"],
  gains_es: ["Lecturas rápidas"],
  costs: ["Stale reads"],
  costs_es: ["Lecturas obsoletas"],
  kinds: ["app", "cache", "sql"],
  practice: ["url-shortener"],
  in_simulator: "Fully: hits, misses and TTL are simulated.",
  in_simulator_es: "Del todo: aciertos, fallos y TTL se simulan.",
};

const card = (id: string, kind: string, flavour: string, x: number, y: number) =>
  ({ id, kind, flavour, label: "", position: { x, y }, props: {}, group_id: null }) as Design["nodes"][number];

export const FAKE_TEMPLATE: Design = {
  schema_version: 2,
  id: "pattern-cache-aside",
  name: "Cache-aside",
  problem_id: null,
  nodes: [card("app", "app", "go", 0, 0), card("cache", "cache", "redis", -150, 170), card("db", "sql", "postgresql", 150, 170)],
  edges: [
    { id: "e1", from: "app", to: "cache", protocol: "query" },
    { id: "e2", from: "app", to: "db", protocol: "query" },
  ],
  groups: [],
  notes: [],
  viewport: { x: 0, y: 0, zoom: 1 },
};

export const FAKE_NUMBERS: NumberFigure[] = [
  { id: "memory-reference", group: "memory", label: "Main memory reference", label_es: "Acceso a memoria principal", value: 100, unit: "ns", note: "RAM is fast.", note_es: "La RAM es rápida.", source: "needed: https://example.org/latency" },
  { id: "rtt-cross-az", group: "network", label: "Round trip between zones", label_es: "Ida y vuelta entre zonas", value: 1.5, unit: "ms", note: "A millisecond or two.", note_es: "Uno o dos milisegundos.", source: "simulator" },
  { id: "app-go-rps", group: "servers", label: "One Go instance", label_es: "Una instancia en Go", value: 12800, unit: "rps", note: "In the simulator.", note_es: "En el simulador.", source: "https://www.postgresql.org/docs/" },
];
