# The tools

Some thirty utilities that run on your machine and call nothing: encoding, hashing, converting, generating, reading a date, calculating, and references.

The *Tools* area (<kbd>Ctrl 2</kbd>) sorts them by category. None of them calls the network: what you paste does not leave the application.

## Finding a tool

- <kbd>Ctrl Shift T</kbd> searches for a tool from any area, and searches the references too.
- A star puts a tool in the *Favourites*; the last ones opened are in *Recent*.
- A tool’s *Example* button fills it in, to see what it does.
- A result is copied as it is shown, or *Raw* where the two differ.
- A tool’s input can also be filled from a note, a snippet or a todo list.

## Keeping a result

*Save as note* files what the tool computed in a snippet, with a title, a space, a folder and tags. Never what was typed into it: a secret pasted into a tool does not land in your notes by mistake.

## Every tool

**Text**

| Tool | What it does |
| --- | --- |
| **Case converter** | camelCase, snake_case, dot.case, Train-Case… and the slug |
| **Line breaks** | CRLF ↔ LF, trailing spaces |
| **Text statistics** | characters, words, lines, bytes, frequencies |
| **Diff** | two texts line by line, or two JSON, YAML, TOML or XML documents compared by their values |

**Crypto**

| Tool | What it does |
| --- | --- |
| **Hash / HMAC** | MD5, SHA-1, SHA-2, SHA-3 |
| **Passwords** | length, character sets, entropy |
| **Identifiers** | UUID, ULID, NanoID, CUID2, ObjectId, KSUID, drawn and read back |
| **JWT** | decoded, its dates read, its HS\* signature verified |

**Encoders**

| Tool | What it does |
| --- | --- |
| **URL** | encode, decode, take apart an address and its parameters |
| **Base64 and other bases** | Base64, Base32, hexadecimal, binary, bytes; an image’s preview |
| **JSON ↔ YAML ↔ TOML ↔ XML** | from one format to another, every way round |
| **Colours** | HEX, RGB, HSL, OKLCH, and the contrast between two colours |

**Generators**

| Tool | What it does |
| --- | --- |
| **JSON** | test data, from a schema or a template document |
| **Fictitious identity** | a test person whose every number passes its check digit |
| **Lorem ipsum** | words, sentences, paragraphs |
| **QR code** | text, URL, Wi-Fi, email or contact, as SVG or PNG |

**Time**

| Tool | What it does |
| --- | --- |
| **Dates and durations** | Unix ↔ ISO 8601, gaps and ISO durations |
| **Time zones** | one time, in every zone chosen |
| **Cron** | read aloud, its next runs, and those between two moments |

**Calculate**

| Tool | What it does |
| --- | --- |
| **Sizes and transfer** | B, kB, MB, GB ↔ KiB, MiB, GiB, bits, and how long a transfer takes |
| **Percentages** | x % of y, share, change, rise and fall |
| **Unix permissions** | chmod: 755 ↔ rwxr-xr-x, special bits, umask |
| **Luhn and IBAN** | a number or an IBAN checked, its right check digits, and valid numbers to generate |
| **CIDR subnets** | masks, hosts, splitting, address plan, IPv4 and IPv6 |

**References**

| Reference | What it holds |
| --- | --- |
| **HTTP status codes** | 404, 429, 503: what they mean, and when to send them |
| **HTTP headers** | Content-Type, Cache-Control, CORS, CSP: what they are for |
| **MIME types** | the type of an extension, and the extensions of a type |
| **Common ports** | 22, 443, 5432, 6379: which service listens where |
| **Unix signals** | SIGTERM, SIGKILL, SIGHUP: numbers, action and use |
| **ASCII table** | the 128 characters, in decimal, hexadecimal, octal and binary |
| **Regex cheat sheet** | the syntax, in JavaScript and PCRE |
| **Markdown cheat sheet** | headings, lists, links, tables, in CommonMark and on GitHub |
| **Mustache cheat sheet** | variables, sections, partials, with what they render |

> [!TIP]
> A tool that opens a file reads it up to 10 MB. Beyond that, it says so rather than cutting it silently.
