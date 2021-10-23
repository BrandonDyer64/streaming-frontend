# streaming-frontend

Consumes an API and displays media data.

## Running

**NOTES:**

- This application has only been tested on **macOS**
- If you see a tile that looks like _Mickey Mouse Clubhouse_ in French, it means the data either hasn't loaded yet or failed to load.
  - This seems to happen on the second tile on the home screen, which sohuld be _The Mandalorian_. It looks like the image it's getting back from the API is a 404.
- The app does dynamically load rows on demand as the user scrolls down the page

### Cargo

This should be all you need to build and run the application. It is entirely possible that there are system dependencies, but I am not aware of any.

```bash
$ cargo run
```

### Binary

- Mac / Linux: `./streaming-service`
- Windows: There is currently no binary for windows. Please use [cargo](#Cargo)

## Screenshots

### On first open

<img width="972" alt="image" src="https://user-images.githubusercontent.com/12021069/138652110-dc5e0bdb-69f9-4357-aabe-92343e4bee25.png">

### Use arrow keys to move around

<img width="972" alt="image" src="https://user-images.githubusercontent.com/12021069/138652345-2027a560-2849-4683-993d-79b04cb5fe58.png">

### Press `Enter` to select a tile

And `Backspace` to go back

<img width="970" alt="image" src="https://user-images.githubusercontent.com/12021069/138652634-30451f6d-f90c-4e87-8563-c698cda483e9.png">
