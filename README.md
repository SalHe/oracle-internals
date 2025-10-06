# Oracle Internal LABs

这个项目主要用于探索与分析Oracle Redo文件的结构，对于物理、逻辑结构的描述，
将使用 [010 Editor](https://www.sweetscape.com/010editor/) 的模板来进行描述。

## 分析方法

1. 编写最小化的测试用例，生成Redo文件，并使用Oracle dump成文本并提取，配合十六进制编辑工具进行分析对比，确定Redo的物理结构。
2. 使用其他资料用来做交叉验证。
