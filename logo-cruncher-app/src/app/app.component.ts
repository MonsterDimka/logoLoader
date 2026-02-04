import {Component, OnInit, OnDestroy, VERSION} from "@angular/core";
import {invoke, convertFileSrc} from "@tauri-apps/api/core";
import {FormsModule, ReactiveFormsModule} from '@angular/forms';
import {listen, UnlistenFn} from '@tauri-apps/api/event';

interface LogoJob {
    id: number;
    url: string;
};

interface Jobs {
    logos: LogoJob[];
}

export interface Root {
    data: Data;
    // status: number;
    // config: Config;
    // statusText: string;
}

export interface Data {
    data: DataItem[];
    total: number;
}

export interface DataItem {
    id: number;
    // created: number;
    // updated: number;
    // username: string;
    // merchantId: number;
    note: string;
    // status: string;
    // priority: string;
    // logo: null;
    // logoAttachment: null;
    attachments: Attachment[];
    // merchant: Merchant;
    // $$hashKey: string;
}

export interface Attachment {
    id: number;
    url: string;
    // $$hashKey: string;
}

@Component({
    selector: "app-root",
    imports: [FormsModule, ReactiveFormsModule],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent implements OnInit, OnDestroy {
    greetingMessage = "Вставьте полный текст copy object из консоли разработчика";
    dirs = "";
    fileList: string[] = [];
    imageUrls: string[] = [];
    jsonText: string = "";
    logos: LogoJob[] = [{id: 34293493, url: "Url 1"}, {id: 342233, url: "Url 2"}];
    angularVersion = VERSION.full;
    subscription: Promise<UnlistenFn> | undefined;


    ngOnInit() {
        this.subscription = listen<String>('event-greet-finished', (event) => {
            console.log(
                `Emmit ${event.payload} ${event}`
            );
        });
    }

    async ngOnDestroy() {
        if (this.subscription) {
            const unlisten = await this.subscription;
            unlisten();
        }
    }

    processJson(event: SubmitEvent, jsonString: string) {
        event.preventDefault();

        console.log("Получен текст:", jsonString);
        try {
            const _ = JSON.parse(jsonString);
            console.log("Это json");
        } catch (error) {
            console.error("Ошибка JSON", error);
            this.greetingMessage = "Ошибка JSON";
            return;
        }
        try {
            const root: Root = JSON.parse(jsonString) as Root;
            console.log("Распарсили структуру json", root.data.data);

            invoke<Jobs>("process_json", {json: jsonString}).then(async (jobs: Jobs) => {
                this.logos = jobs.logos;
                console.log("logos", this.logos);
                // this.greetingMessage = this.logos.join(", ");
            }).catch((err) => {
                console.error(err);
                this.greetingMessage = "Ошибка: " + String(err);
            });

        } catch (error) {
            console.error("Ошибка типа JSON", error);
            this.greetingMessage = "Ошибка типа JSON";
            return;
        }


        // invoke<Jobs>("process_json", {json})
        //     .then(async (jobs: Jobs) => {
        //         this.logos = jobs.logos;
        //         console.log("logos", this.logos);
        //         // this.greetingMessage = this.logos.join(", ");
        //     }).catch((err) => {
        //     console.error(err);
        //     this.greetingMessage = "Ошибка: " + String(err);
        // });

        // invoke<string>("process_json", {json: this.jsonText})
    }

    greet(): void {
        let name = this.logos.map((value, index) => index + ") id:" + value.id + " url:" + value.url).join(",");

        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        invoke<string>("greet", {name}).then(async (text) => {
            this.greetingMessage = text;

            // await this.loadFileList();
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

        // invoke<string>("logo_list", {msg: this.dirs}).then(async (dirs) => {
        //     this.greetingMessage = dirs;
        // });


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
