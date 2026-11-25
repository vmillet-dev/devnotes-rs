# The API client

Sending HTTP and GraphQL requests, opening WebSockets, filing them in collections, and varying them from one environment to another.

The *HTTP* area (<kbd>Ctrl 3</kbd>) keeps your requests in collections, sealed like your notes. A request goes to the server you name, and nowhere else.

## Starting a request

*+ New request*, at the end of the tabs or in a collection’s menu, asks what you are starting:

- **HTTP request**: a method, a URL, headers and a body.
- **GraphQL request**: a query and its variables, sent as a POST.
- **WebSocket**: an open connection, messages both ways.

Each request opens in a tab. <kbd>Ctrl Enter</kbd> sends it (or connects the WebSocket), <kbd>Ctrl S</kbd> saves it in a collection. Tabs are dragged to change their order, and a middle click closes one.

### The parts of a request

- **Parameters** and **Headers**: key / value tables, each row can be switched off. Those DevNotes adds by itself are shown apart.
- **Auth**: *Inherited*, *None*, *Basic*, *Bearer* or *API key*, sent in a header or in the query.
- **Body**: JSON, text, form, multipart or file. A file is read when the request is sent, its bytes are never kept.
- **Settings**: the timeout, redirects, TLS certificate verification and cookies.

### The response

The status, the duration and the size, then the body *Formatted* or *Raw*; a JSON also reads as a *Graph* or a *Tree*. Beside it, the *Headers*, the *Cookies* set and the *Timeline* of the exchange. A response is saved to a file or kept as a note.

## Collections and folders

A collection files requests, in folders if you like. Its *Settings…* pass down to its requests: a request whose auth is *Inherited* takes its folder’s or its collection’s, and their headers are added to its own.

**In the collections rail**

| Keys | Action |
| --- | --- |
| <kbd>Alt ↑ ↓</kbd> | Move a row among its neighbours |
| <kbd>Alt →</kbd> | File it in the folder just above |
| <kbd>Alt ←</kbd> | Take it out of its folder |
| <kbd>F2</kbd> | Rename |
| <kbd>Delete</kbd> | Delete, after confirmation |

## Variables and environments

A variable is written between double braces, anywhere in the request:

**A URL that varies with the environment**

```
GET {{base_url}}/v1/invoices?status=paid
```

It is resolved when the request is sent, from nearest to farthest: the request, its folder, its collection, the active environment, then the globals. Under the URL, each name shows, when pointed at, its value and where it comes from. A name nothing defines blocks the send rather than going out as it is, and *Set in …* opens the place to define it.

1. *Manage environments…* creates Local, Staging, Production: each a set of variables, with a colour.
2. The active environment is chosen at the end of the tabs. *None* keeps the globals alone.
3. A variable is *Text*, a *Number* or a *Secret*: a secret is masked on screen and never written to the history.

> [!WARNING]
> A **protected** environment asks before sending anything other than a GET, a HEAD or an OPTIONS, and before opening a WebSocket. Put it on production.

> [!TIP]
> A snippet’s `{{fields}}` and a request’s variables are written alike, but do not mix: the former are asked for on copy, the latter resolved on send.

Five names are made when the request leaves, a new value each time: `{{$guid}}` (and `{{$randomUUID}}`), `{{$timestamp}}`, `{{$isoTimestamp}}` and `{{$randomInt}}`. No level can define one of them.

## Scripts and tests

A request, a folder or a collection can run JavaScript *Before the send* and *After the response*, and hold named *Tests* checked after the response. The scripts run in a process of their own, with no access to your files, and are stopped if they take too long or use too much memory.

- Before the send, a script changes the request or reads and writes a variable: add a header, rewrite the body, skip the request. A script that fails stops the send, and says so.
- After the response, a script reads the response and keeps a value of it in a variable, for the next request.
- A test is a named check: its result goes to the *Tests* tab, with what was expected and what was received.
- The collection’s scripts run first before the send, and last after the response, the request’s own being nearest to the request.

**A test after the response**

```js
pm.test('The status is 200', () => {
  pm.response.to.have.status(200);
});
```

The scripts speak Postman’s `pm` object: variables, globals, the request and the response, `pm.test`, `pm.expect` and `pm.sendRequest`. The *Examples* menu inserts a few at the end of the script.

### The console

The *Console* shows what the request sent, what the scripts print, and their errors. It can be filtered, folded or shared with the response, and a line opens the exchange it comes from.

## Running a collection

A collection or a folder runs request after request, as many iterations as you ask. A data file, CSV or JSON, supplies one row of variables per iteration. The order of the requests is yours to change (drag, or <kbd>Alt ↑ ↓</kbd> on the row), and <kbd>Space</kbd> leaves one out.

- *Stop at the first failure* ends the run at the first failed test.
- *Keep the variables changed* and *Keep the responses* say what survives the run.
- A protected environment asks before the run changes anything on it.

The results list every request of every iteration, with its failures, and can be filtered.

## History

Every send is recorded in the *History*, encrypted like your notes: 30 days, 500 sends at most. A send can be read again, reopened in its request or as a draft, and erased. Secrets are masked before they are written.

## A request’s code

The *</> Code* button writes the request for twenty targets, from cURL and PowerShell to fetch, axios or Angular’s HttpClient. Variables can be resolved in it; secrets stay as variables, unless you ask to include them.

## WebSockets

A WebSocket connects, then exchanges: the log shows every message sent and received, with its time. Messages can be kept to send again. Switching library closes the open connections.
