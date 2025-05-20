import { invoke } from "@tauri-apps/api/core";

function App() {
  return (
    <>
      <div>
        <h1>fullscreen+setframe</h1>
        <button onClick={async () => {
          await invoke('start', { isFullscreen: true, setFrame: true })
        }}>enter</button>
        <button onClick={async () => {
          await invoke('stop', { isFullscreen: true, setFrame: true })
        }}>exit</button>
      </div>
      <div>
        <h1>only fullscreen</h1>
        <button onClick={async () => {
          await invoke('start', { isFullscreen: true, setFrame: false })
        }}>enter</button>
        <button onClick={async () => {
          await invoke('stop', { isFullscreen: true, setFrame: false })
        }}>exit</button>
      </div>
      <div>
        <h1>only setframe</h1>
        <button onClick={async () => {
          await invoke('start', { isFullscreen: false, setFrame: true })
        }}>enter</button>
        <button onClick={async () => {
          await invoke('stop', { isFullscreen: false, setFrame: true })
        }}>exit</button>
      </div>
    </>
  );
}

export default App;
