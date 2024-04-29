# esp32c3 mqtt demo

通过`dht11`crate读取dht11的温湿度数据，上传到mqttx免费broker,再通过app订阅该主题，通过echarts展示温湿度历史数据，

- 修改rust-mqtt部分源码适配
- `dht11`适配`embedded_hal 1.0`
