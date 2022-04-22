## How does mortar work?
Saffron app creates swagger definition, with occasional helper attributes added to aid FE action creation (x-mtr).

Mortar is run as a single command, reads the settings file `mortar.toml` in the FE portal root folder.

It calls the backend to get the swagger json, which is parsed and generates redux action creators and type definitions.

# mortar.toml file

```toml
debug = false
swagger_endpoint = "http://localhost:5000/api/swagger.json"
mortar_endpoint = "http://localhost:5000/mortar/buildId"
output_dir = "./app/mortar"
```


Running mortar
`mortar` will run once and exit.
`mortar --watch` will rebuild types any time the backend restarts