<script setup>
import * as echarts from "echarts";
import {
  TitleComponent,
  ToolboxComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
} from "echarts/components";
import { LineChart } from "echarts/charts";
import { UniversalTransition } from "echarts/features";
import { CanvasRenderer } from "echarts/renderers";
import WebSocket from "@tauri-apps/plugin-websocket";
import { invoke } from "@tauri-apps/api/core";
import { ref, onMounted, onBeforeUnmount } from "vue";

echarts.use([
  TitleComponent,
  ToolboxComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  LineChart,
  CanvasRenderer,
  UniversalTransition,
]);

const colors = ["#EE6666", "#5470C6"];
const x_date = [
  "00",
  "01",
  "02",
  "03",
  "04",
  "05",
  "06",
  "07",
  "08",
  "09",
  "10",
  "11",
  "12",
  "13",
  "14",
  "15",
  "16",
  "17",
  "18",
  "19",
  "20",
  "21",
  "22",
  "23",
];
const myChart = ref({});
const tempData = ref([]);
const humiData = ref([]);

const thChartOption = ref({
  color: colors,
  tooltip: {
    trigger: "axis",
    axisPointer: {
      type: "cross",
    },
  },
  grid: {
    right: "20%",
  },
  toolbox: {
    feature: {
      dataView: { show: true, readOnly: false },
      restore: { show: true },
      saveAsImage: { show: true },
    },
  },
  legend: {},
  xAxis: [
    {
      type: "category",
      axisTick: {
        alignWithLabel: true,
      },
      data: x_date,
    },
  ],
  yAxis: [
    {
      type: "value",
      name: "温度",
      position: "left",
      alignTicks: true,
      axisLine: {
        show: true,
        lineStyle: {
          color: colors[0],
        },
      },
      axisLabel: {
        formatter: "{value} °C",
      },
    },
    {
      type: "value",
      name: "湿度",
      position: "right",
      alignTicks: true,
      axisLine: {
        show: true,
        lineStyle: {
          color: colors[1],
        },
      },
      axisLabel: {
        formatter: "{value} %rh",
      },
    },
  ],
  series: [
    {
      name: "Temperature",
      type: "line",
      yAxisIndex: 0,
      data: tempData.value,
    },
    {
      name: "Humidity",
      type: "line",
      yAxisIndex: 1,
      data: humiData.value,
    },
  ],
});

onMounted(() => {
  // 基于准备好的dom，初始化echarts实例
  myChart.value = echarts.init(document.getElementById("th"));
  // 绘制图表
  myChart.value.setOption(thChartOption.value);
});

onBeforeUnmount(() => {
  clearInterval(timer);
});

const tempHumiApi = async () => {
  let res = await invoke("temp_humi", {});

  let ts = res.map((th) => {
    return th.t;
  });
  let hs = res.map((th) => {
    return th.h;
  });

  tempData.value = Array.from(ts);
  humiData.value = Array.from(hs);

  myChart.value.setOption({
    series: [
      {
        name: "Temperature",
        data: tempData.value,
      },
      {
        name: "Humidity",
        data: humiData.value,
      },
    ],
  });
};

const timer = setInterval(() => {
  tempHumiApi();
}, 3000);
</script>

<template>
  <div>
    <div id="th" :style="{ width: '600px', height: '500px' }"></div>
  </div>
</template>

<style scoped></style>
