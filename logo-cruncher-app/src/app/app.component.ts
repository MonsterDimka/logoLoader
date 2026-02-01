import {Component} from "@angular/core";
import {RouterOutlet} from "@angular/router";
import {invoke} from "@tauri-apps/api/core";
import {open} from '@tauri-apps/plugin-dialog';

@Component({
    selector: "app-root",
    imports: [RouterOutlet],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent {
    greetingMessage = "";
    dirs = "";
    fileList: string[];

    constructor() {
        this.fileList = ["None"];
    }

    greet(event: SubmitEvent, name: string): void {
        event.preventDefault();


        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        invoke<string>("greet", {name}).then(async (text) => {
            this.greetingMessage = text;

            await this.loadFileList();
            // const file = await open({
            //     multiple: false,
            //     directory: true,
            // });
            //
            // if (file) {
            //     this.greetingMessage = this.greetingMessage + file;
            //     this.dirs = file;
            // }
            // console.log(file);
        });

        invoke<string>("logo_list", {msg: this.dirs}).then(async (dirs) => {
            this.greetingMessage = dirs;
        });


    }

    // Пример вызова
    async loadFileList() {
        try {
            const files: string[] = await invoke("get_file_list");
            console.log("Список файлов:", files);
            this.fileList = files;  // например, в свойство компонента
        } catch (error) {
            console.error("Ошибка:", error);
        }
    }
}
