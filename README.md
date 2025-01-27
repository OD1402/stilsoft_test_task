### О программе

stilsoft_test_task выполняет следующие действия:
* получает на вход список URL
* скачивает содержимое страниц в асинхронном режиме, 
* считает количество символов в первой строке
* складывает полученные результаты в БД sled
* выводит результат обработки на экран

### Настройка окружения

* Установить rust - https://www.rust-lang.org/ru/tools/install
* Склонировать репозиторий `git clone git@github.com:OD1402/stilsoft_test_task.git`

### Варианты запуска

1. С передачей ссылок через файл
     * Указать список ссылок, которые нужно обработать, в файл `~stilsoft_test_task/source.json`
     * Запустить команду `cargo run`
       
2. С передачей ссылок через командную строку
     * Запустить команду, указав ссылки для обработки через пробел
       <br/>`cargo run -- https://example.com https://zipal.ru/export/agency/8562/YANDEX https://www.rust-lang.org`
       
<br/>Если при запуске указаны ссылки как аргументы командной строки, то ссылки в файле `~stilsoft_test_task/source.json` будут проигнорированы.

### Вывод результатов обработки

Результаты обработки выводятся на экран терминала, пример вывода:
```
====================
Обработка завершена. 
results: {
    "https://www.rust-lang.org": 15,
    "https://www.youtube.com/?app=desktop&hl=ru": 1285,
    "https://www.cian.ru/metros-moscow-v2.xml": 38,
    "https://example.com": 15,
    "http://xapi.ru/xml/L0q0n0d0m2j0m0r0k04231z232k1u272h1s222h222k250s060c0k0e0v0p120w1q1b1g1a153c393j1c3j1c1j.xml": 3850,
    "http://base.tsnnedv.ru/Publisher/sob/3?region=3": 38,
}
====================
```
