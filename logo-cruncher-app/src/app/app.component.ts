import {Component} from "@angular/core";
import {RouterOutlet} from "@angular/router";
import {open} from '@tauri-apps/plugin-dialog';
import {invoke, convertFileSrc} from "@tauri-apps/api/core";
import {FormsModule} from '@angular/forms';

type LogoJob = {
    id: number;
    url: string;
};

type Jobs = {
    logos: LogoJob[];
}

@Component({
    selector: "app-root",
    imports: [FormsModule],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent {
    greetingMessage = "Вставьте полный текст copy object из консоли разработчика";
    dirs = "";
    fileList: string[] = [];
    imageUrls: string[] = [];
    jsonText: string = "";
    jobList: Jobs = {logos: []};

    processJson() {
        console.log("this.jsonText", this.jsonText);
        invoke<Jobs>("process_json", {json: this.jsonText}).then(async (text: Jobs) => {
            this.greetingMessage = "dfdfgdfgf";
            this.jobList = text;
        });

        // invoke<string>("process_json", {json: this.jsonText})
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

    isImageFile(path: string): boolean {
        return /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(path);
    }

    async loadFileList() {
        try {
            const files: string[] = await invoke("get_file_list");
            this.fileList = files;
            this.imageUrls = files
                .filter(p => this.isImageFile(p))
                .map(p => convertFileSrc(p));
        } catch (error) {
            console.error("Ошибка:", error);
        }
    }
}
