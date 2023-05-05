import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event"
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/api/notification';
import "./index.css";
import "./App.css";

function App() {
  const [password, setPassword] = useState("");

  useEffect(() => {
    const unlistenWrongPass = listen('wrong_password', () => {
      (async () => {
        let permissionGranted = await isPermissionGranted();
        if (!permissionGranted) {
            const permission = await requestPermission();
            permissionGranted = permission === 'granted';
        }
        if (permissionGranted) {
            sendNotification('密码错误，请重新输入.');
        }
      })()
      
      setPassword("")
    })

    const unlistenSucess = listen('launch_success', () => {
      (async () => {
        let permissionGranted = await isPermissionGranted();
        if (!permissionGranted) {
            const permission = await requestPermission();
            permissionGranted = permission === 'granted';
        }
        if (permissionGranted) {
            sendNotification('PowerMetrics 启动成功.');
        }
      })()
    })

    return () => { 
      unlistenWrongPass.then((f) => f());
      unlistenSucess.then((f) => f());
    };
  }, [])

  async function submit() {
      await invoke("start_mertics", { password })
  }

  return (
    <div className=" bg-gray-100 w-fit h-fit p-4">
      <div className=" w-fit mx-auto block my-2 font-light text-s">运行 PowerMetrics Menu 需要当前用户密码，仅用于获取硬件数据</div>
      <div className="m-auto w-fit">
        <input type="password" name="password" value={password} onChange={(e) => setPassword(e.target.value)} className="mx-3 my-2"></input>
        <button type="button" onClick={submit} className="mx-2 text-s font-light transition-all bg-white hover:bg-slate-300 px-2 py-0.5 rounded-md">完成</button>
      </div>
    </div>
  );
}

export default App;
