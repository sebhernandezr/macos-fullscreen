import { invoke } from "@tauri-apps/api/core";

function App() {
  return (
    <>
      <button onClick={async () => {
        await invoke('start')
      }}>fullscreen</button>
      <button onClick={async () => {
        await invoke('stop')
      }}>exit</button>
    </>
  );
}

export default App;
