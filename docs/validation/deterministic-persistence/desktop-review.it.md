# Revisione desktop della PR #1

Il desktop ora accetta lo schema 8 e conserva il seed zero negli schemi 1–8.
Lo schema 0 con seed zero resta ambiguo, coerentemente con il fallback dichiarato
nel caricatore Rust: non viene trattato come un'identità stabile durante import,
spostamento o conferma del mondo caricato.

## Correzioni

- `save-validation.ts`: supporto v8; griglia 600×300 ammessa solo per gli schemi
  legacy 0–7, perché lo schema corrente richiede 800×400; regola del seed
  centralizzata in `hasStableWorldSeed` e usata nel confronto del mondo caricato.
- `save-folder-migration.ts`: stessa regola sia nell'ispezione sincrona sia nel
  worker, senza riscrivere i byte del checkpoint.
- `ipc.ts`: anche la conferma del mondo nuovo dopo reset o migrazione da una
  cartella vuota accetta un seed zero esplicito negli schemi che lo supportano.
- `world-safety.ts`: espone la regola comune ai chiamanti desktop.

## Percorsi verificati

| Percorso | Verifica |
| --- | --- |
| Validazione sincrona e worker | Checkpoint Rust v8 completo, conservazione esatta dei byte |
| Helper di download/import | Risposta HTTP simulata con v8; rifiuto v9; nessuna riserializzazione |
| Identificazione della cartella dati | Sia albero `worlds/<hash>` sia file `world.save` nella radice |
| Controllo dopo caricamento/import | Zero valido v1–v8, zero ambiguo v0 rifiutato; seed diverso e tick arretrato rifiutati |
| Esportazione IPC | Handler reale, scrittura transazionale e confronto di tutti i byte |
| Reset con esportazione IPC | Esportazione v8 e conferma di un nuovo mondo con seed zero |
| Migrazione IPC della cartella | Worker reali, copia, lock, verifica del caricamento, aggiornamento impostazioni e chiusura del journal |
| Migrazione cartella v7/v8 con zero | Ispezione sincrona e worker; sorgente preservata |
| Ripresa Rust dopo trasferimento desktop | Checkpoint maturo 6.000 → 6.100, identico al riferimento continuo |

Il trasferimento usa i byte originali. Il JSON viene letto per i controlli, ma non
viene riserializzato: così seed u64 e stato RNG u128 non perdono precisione.
La fixture v8 contiene stream `18446744073709551615` e word position
`36893488147419103239`; origine e hash sono documentati accanto alla fixture.

Non è stata aggiunta una nuova funzione di importazione nell'interfaccia: è stato
verificato l'helper di download esistente e il controllo di caricamento usato
nella migrazione. La validazione completa dello stato resta responsabilità Rust;
il desktop mantiene il controllo preliminare su formato, versione e metadati.

## Test effettivamente eseguiti

Comando richiesto, da `apps/desktop`:

```sh
pnpm test:main
```

**Exit code 0: 222 test passati, 0 falliti, 0 saltati.** Nove sono nuovi test di
regressione. Lo script esegue prima `pnpm run build:main`, quindi
`tsc -p tsconfig.json`: compilazione TypeScript di main e preload riuscita.
Superato anche `git diff --check`.

Inoltre è stata eseguita una verifica integrata con il binario Rust già compilato
al commit `7d49cd6`, senza modificarlo né ricompilarlo:

1. Validazione worker e trasferimento della cartella con il checkpoint reale
   seed 42 / tick 6.000 tramite i moduli desktop compilati.
2. Verifica byte per byte della copia e caricamento con `headless --load ...
   --ticks 6100 --every 1000 --save ...`.
3. Confronto completo con il riferimento continuo già conservato, verifica
   seed/tick nel desktop e chiusura del journal di migrazione.

**Esito: identità byte per byte al tick 6.100**, SHA-256 comune:
`99777e876c1af6762e9765143d2ab8628d2a91da5c650bb2edfd48c609233b30`.

## Ambito e limiti

Nessun file Rust, algoritmo della simulazione o test di continuità preesistente
è stato modificato. La precedente suite Rust da 1.182 test passati non è stata
ripetuta: il codice Rust è invariato; la prova aggiuntiva riguarda il nuovo
percorso desktop → Rust.

Ambiente: Linux x86_64, Node 24.22.1, pnpm 11.25.0, TypeScript 5.9.3 installato
dal lockfile senza aggiornarlo. I test IPC usano filesystem, lock e worker reali,
ma simulano finestre/dialoghi Electron, impostazioni e processo della simulazione.
La prova integrata separata usa invece il vero binario Rust. Non sono stati
eseguiti test grafici, packaging Electron, firma installer o prove su Windows/MSI.
