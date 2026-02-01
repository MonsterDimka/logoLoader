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

    greet(event: SubmitEvent, name: string): void {
        event.preventDefault();


        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        invoke<string>("greet", {name}).then(async (text) => {
            this.greetingMessage = text;

            const file = await open({
                multiple: false,
                directory: true,
            });

            if (file) {
                this.greetingMessage = this.greetingMessage + file;
                this.dirs = file;
            }
            console.log(file);
        });

        invoke<string>("logo_list", {msg: this.dirs}).then(async (dirs) => {
            this.greetingMessage = dirs;
        });
    }
}
