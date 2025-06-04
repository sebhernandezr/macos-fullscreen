import { invoke } from "@tauri-apps/api/core";

function App() {
  return (
    <>
      <div>
        <h1>fullscreen+setframe</h1>
        <button onClick={async () => {
          await invoke('start', {})
        }}>enter</button>
        <button onClick={async () => {
          await invoke('stop', {})
        }}>exit</button>
      </div>
    </>
  );
}

export default App;
