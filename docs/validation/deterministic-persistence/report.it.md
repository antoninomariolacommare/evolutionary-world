# Continuità deterministica dei checkpoint — Implementazione 1

Repository: `antoninomariolacommare/evolutionary-world`.
Base: `4667dc190f1a3531466d3601e55d5a01d6d97711`.
Branch: `fix/deterministic-persistence`.

**Esito tecnico: continuità verificata nei casi di accettazione eseguiti; suite
workspace completata con codice 0, 1.182 test passati e nessun fallimento.**

## Diagnosi e correzione

Il vecchio checkpoint non descriveva tutto lo stato necessario alla prosecuzione.
Inoltre il caricatore applicava riparazioni anche a uno stato appena salvato.
La riproduzione iniziale, eseguita prima delle modifiche, rileva 20 campi principali
diversi al tick 6.100. La posizione RNG passa da 88.968.405 nell'esecuzione continua
a 88.979.226 in quella ripresa. I comandi, gli esiti e gli hash dei file originali
sono in `before.json` e `baseline-files.json`.

| Causa verificata nel codice | Intervento |
| --- | --- |
| Normalizzazione del bordo oceanico al caricamento | Solo migrazione; fertilità corrente conservata |
| Vocabolari e memorie vuoti interpretati come mancanti | Conservazione esatta nello schema corrente |
| Ricalcolo anticipato degli insediamenti, riparazioni di religioni, infrastrutture e contatori | Separazione dal ripristino corrente |
| `next_battle_id` e altri contatori corretti automaticamente | Ripristino del valore salvato |
| Heading e attività degli animali omessi | Persistenza esplicita |
| Route, obiettivi bloccati, lavoro, danni recenti e velocità smussata omessi | Sezione runtime individuale |
| Abilità, sensazioni e lezioni arrotondate nelle mappe di esportazione | Array numerici esatti autorevoli, mappe precedenti mantenute come proiezioni |
| Cooldown, censimenti, traghetti, cimiteri, aggregati e altri stati temporali omessi | Sezione runtime del mondo |
| Proprietario più recente di territori sovrapposti non ricostruibile | Salvataggio della mappa inversa effettiva |
| `Building.last_work_tick` escluso da Serde | Persistenza del tick che determina l'abbandono dei cantieri |
| Configurazione demografica persa | Salvataggio del limite di popolazione |
| Seed zero trattato come assente | Zero valido per gli schemi con seed dichiarato |
| RNG assente sostituito da un nuovo generatore | Errore per schema corrente; fallback dichiarato solo nella migrazione |
| Code storiche accorciate durante il salvataggio | Nessun taglio aggiuntivo oltre alla retention già applicata dal motore |

Il vecchio formato già serializzava ChaCha8 quando presente: la divergenza RNG
osservata dopo la ripresa non dimostra un difetto di ChaCha, ma è compatibile con
un diverso consumo di casualità causato dallo stato alterato. Il nuovo validatore
vieta anche il fallback quando manca RNG in uno schema corrente.

I test intermedi hanno localizzato ulteriori omissioni: dopo aver conservato la
memoria di movimento, individui e RNG coincidevano, ma al tick 6.100 divergevano
edifici ed eventi. Il tick di lavoro non serializzato partecipava alla decisione
di abbandonare un cantiere. Non è stato modificato quell'algoritmo.

## Contratto e file modificati

Lo schema passa da 7 a 8. `from_checkpoint_bytes` valida l'input e conserva l'ordine
delle mappe deserializzando i byte originali. I checkpoint correnti incompleti
vengono rifiutati; quelli precedenti seguono il percorso di migrazione dichiarato.
`load_strict` non modifica il file di input. `load_or_new` resta la politica
esplicita di recupero: conserva il file rifiutato prima di creare un mondo nuovo.
Il dettaglio, incluse le inizializzazioni delle migrazioni, è nel
[contratto](../../deterministic-persistence.md).

- `sim/storage/persistence/`: schema, cattura/ripristino, validazione, migrazione,
  adattatore per mappe con chiavi tuple e test di continuità.
- `organism/`, `physics/engine.rs`, `sim/tech/ferry.rs`, `sim/agents/daily_work.rs`,
  `sim/civ/divine/peril.rs`, `sim/civ/society/settlements.rs`, `sim/simulation/`:
  metadati di serializzazione, visibilità interna e documentazione dello stato.
- `sim/tech/buildings/building.rs`: serializzazione di `last_work_tick`.
- `lib.rs`: anche l'importazione WASM usa lo stesso validatore; nessuna modifica
  all'interfaccia grafica.
- Test precedenti che richiedevano riparazioni: marcati esplicitamente come
  migrazioni legacy. Il test del clock fisico ora richiede il valore effettivo
  salvato, non un valore ricavato dal clock globale.

ChaCha8, codifica numerica dei layer, scrittura temporanea, sincronizzazione e
pubblicazione atomica restano in uso. Nessun algoritmo sociale, cognitivo,
economico o culturale è stato riscritto. Nessun codice Common Ground è stato usato.

## Limiti e rischi

- Garanzia di continuità verificata solo per lo stesso motore e ambiente Linux
  x86_64 di questa prova. Windows/MSI e WASM richiedono verifica sul target.
- I vecchi salvataggi non contengono le informazioni omesse: una migrazione non
  può ricostruire retroattivamente la loro traiettoria originale.
- L'API `from_save` accetta uno stato Rust già decodificato e fidato. I chiamanti
  che leggono dati esterni devono usare `from_checkpoint_bytes` per riconoscere
  campi mancanti prima che Serde applichi i default legacy.
- I campi volutamente sparsi hanno la semantica di default del formato. La
  validazione strutturale non è un checksum e non rileva ogni alterazione dei
  contenuti. Non è stato implementato un formato con verifica crittografica.
- Maggiori dimensioni e allocazioni al caricamento: la validazione costruisce
  alberi JSON oltre allo stato tipizzato. Nessun benchmark prestazionale sull'MSI.
- Nessun archivio permanente: restano le politiche di retention del motore.
- La politica preesistente del server sulla pressione di memoria può modificare
  la cognizione. Gli esperimenti richiedono la stessa sequenza di input e politiche
  esterne; questa attività non modifica tale comportamento.
- Cache di rete, contatori di report e cronaca solo visiva restano transitori.
  L'indice spaziale degli edifici e le maschere delle scoperte sono ricostruiti
  dai dati ordinati; le prove di continuità esercitano il loro utilizzo successivo.
  Le cache con memoria temporale, invece, sono salvate.

Il rischio principale di regressione riguarda la completezza dello schema quando
si aggiungeranno nuovi campi: ogni nuovo stato comportamentale dovrà entrare nel
checkpoint e nei test. La compatibilità legacy è deliberatamente una migrazione,
non una promessa di equivalenza tra versioni del motore.

## Confronto headless prima/dopo

Nove esecuzioni del nuovo binario, tutte terminate con codice 0, hanno prodotto
questi confronti sul checkpoint completo e sullo stato RNG:

| Prova | Risultato |
| --- | --- |
| Salva a 6.000, carica e risalva senza tick | Identità byte per byte |
| Continuo 6.001 vs checkpoint 6.000 + 1 tick | Identità byte per byte |
| Continuo 6.100 vs checkpoint 6.000 + 100 tick | Identità byte per byte |
| Migrazione del vero checkpoint v7 a 6.000, poi ripresa v8 fino a 6.100 | Identità byte per byte rispetto alla prosecuzione diretta della migrazione |
| Traiettoria continua originale vs nuova | Stessi valori su tutti i campi del formato originale, dopo la sola normalizzazione delle differenze di schema/esportazione |

La normalizzazione dell'ultimo confronto rimuove i nuovi campi, riporta il numero
di schema a 7, applica i vecchi limiti delle code storiche e riordina la vecchia
lista di wariness. Non modifica posizioni, decisioni, relazioni, risorse o RNG.
Non confronta i nuovi campi con informazioni che il vecchio file non conteneva.
I confronti di continuità v8 non usano alcuna normalizzazione.

Hash SHA-256 comune al tick 6.100:
`99777e876c1af6762e9765143d2ab8628d2a91da5c650bb2edfd48c609233b30`.

I dettagli con comandi, codici di uscita, dimensioni e hash sono in `after.json`.
I file di simulazione originali e i log sono conservati nella cartella di lavoro
`/workspace/work/implementation-evidence`.

## Dimensioni e prestazioni

Al tick 6.000 il file passa da 19.479.908 a 23.037.068 byte: +3.557.160 byte,
circa +18,3%. È il costo osservato di questo mondo, non una stima generale per
popolazioni numerose. I tempi wall registrati comprendono contesa con la suite
di test; non sono un benchmark comparabile prima/dopo. L'impatto su latenza,
memoria di picco e MSI non è stato misurato in condizioni controllate.

Ambiente effettivo: Linux x86_64, Rust 1.100.0, quota CPU equivalente a 2 core,
limite cgroup di memoria 8 GiB. Nessuna verifica effettuata sul computer MSI.

## Test effettivamente eseguiti

`cargo test --release --locked --workspace` — **exit code 0**.

| Target | Passati | Falliti | Ignorati |
| --- | ---: | ---: | ---: |
| Headless | 10 | 0 | 0 |
| sim-core | 1.139 | 0 | 1 |
| Server | 33 | 0 | 1 |
| Doc-test | 0 | 0 | 0 |
| Totale | 1.182 | 0 | 2 |

I sei nuovi test di regressione passano: confronto completo a tick 6.000/6.001/
6.100; seed 0/1/7 con configurazione personalizzata e attraversamento di cadenze;
stati individuali, relazioni, animali, navigazione, cantieri e cooldown non banali;
rifiuto di campi mancanti anche annidati; migrazione ripetibile e ripresa corrente;
stream RNG non predefinito e posizione maggiore di u64 attraverso più refill.

I due test ignorati erano già dichiarati on-demand nel repository e **non sono
stati eseguiti**: `site_search_matches_the_single_site_rules_to_tick_18000` e
`measure_frame_egress_over_mature_worlds`. Il doc-test target contiene zero test.

Superati anche `cargo fmt --all -- --check` e `git diff --check`.
Il riepilogo macchina e l'hash del log completo sono in `tests.json`.
