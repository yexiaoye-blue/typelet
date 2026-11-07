# 导入文章和词表

导入在词本页完成：先预览，确认后才写入本机。网页预览不能模拟文件选择。桌面端把导入结果和当前练到的条目记在应用数据目录的 `content.json` 里，不保存打到一半的按键。

## 句子

UTF-8 纯文本，扩展名 `.txt`。一个文件是一篇文章，文件名作为标题。不使用 JSON 或表格包装。

解析时去掉 BOM，把换行收成空格，再按 `.` `!` `?` `。` `！` `？` 切句。小数点、`Mr.` `Mrs.` `Ms.` `Dr.` `Prof.` `Sr.` `Jr.` `St.` `vs.` `e.g.` `i.e.` `etc.` 不会被切开。句末的引号和括号留在当前句里。正文为空时导入失败。导入的文章整篇算一章。

~~~text
You have power over your mind—not outside events. Realize this, and you will find strength.

The happiness of your life depends upon the quality of your thoughts.
~~~

不直接导入 EPUB、PDF、DOCX 或 HTML。

## 单词

UTF-8 制表符分隔文本，扩展名用 `.tsv`，也接受 `.txt`。第一行必须是表头。

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| name | 是 | 英文单词。不能为空，也不能包含空格 |
| trans | 否 | 释义。多个释义用 `；` 或 `;` 分开 |
| usphone | 否 | 美式音标 |
| ukphone | 否 | 英式音标 |

只接受这四个字段，字段顺序由表头决定。重复字段、缺少 `name`、某一行字段比表头多，都会报错并带上行号。同名单词保留第一次出现的那行，导入结果里会写出重复数量。没有词条时不导入。导入的词表默认每 20 个词一章，可在设置中调整每章数量。

~~~text
name	trans	usphone	ukphone
abandon	放弃；遗弃	əˈbændən	əˈbændən
ability	能力；才能	əˈbɪləti	əˈbɪləti
~~~

只有单词时可以只留 `name` 列：

~~~text
name
abandon
ability
~~~

不导入 Anki `.apkg`，也不把 Qwerty Learner 的 JSON 词库当作导入格式。

[返回使用说明](../README.md)
