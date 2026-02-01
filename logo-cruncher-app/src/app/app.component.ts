import {Component} from "@angular/core";
import {RouterOutlet} from "@angular/router";
import {open} from '@tauri-apps/plugin-dialog';
import {invoke, convertFileSrc} from "@tauri-apps/api/core";
import {FormControl, FormGroup, ReactiveFormsModule} from '@angular/forms';
import {ButtonModule} from "primeng/button";

@Component({
    selector: "app-root",
    imports: [RouterOutlet, ReactiveFormsModule, ButtonModule],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent {
    greetingMessage = "";
    dirs = "";
    fileList: string[] = [];
    imageUrls: string[] = [];
    jsonJob: string = "";

    feedbackForm = new FormGroup({
        reason: new FormControl('Initial reason') // Form control with default value
    });

    onSubmit() {
        console.log(this.feedbackForm.value.reason);
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
