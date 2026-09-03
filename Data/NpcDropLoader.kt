package tw.tsm.data.loaders

import io.github.oshai.kotlinlogging.KotlinLogging
import kotlinx.serialization.json.*
import tw.tsm.domain.data.NpcDropDef
import java.nio.file.Files
import java.nio.file.Path

private val logger = KotlinLogging.logger {}

/**
 * NPC 掉落表載入器
 *
 * 從 `gamedata/npc_drops.json` 載入 NPC 掉落定義。
 * JSON 格式：
 * ```json
 * {
 *   "1001": [
 *     { "itemId": 100, "rate": 50, "minQty": 1, "maxQty": 3 }
 *   ]
 * }
 * ```
 */
object NpcDropLoader {

    /**
     * 載入 NPC 掉落表
     *
     * @param path npc_drops.json 的路徑
     * @return npcId → 掉落列表 映射
     */
    fun load(path: Path): Map<Int, List<NpcDropDef>> {
        if (!Files.exists(path)) {
            logger.warn { "NPC 掉落表不存在：$path，使用空掉落表" }
            return emptyMap()
        }

        val text = Files.readString(path)
        val json = Json.parseToJsonElement(text).jsonObject
        val result = mutableMapOf<Int, List<NpcDropDef>>()

        for ((npcIdStr, dropsElement) in json) {
            val npcId = npcIdStr.toIntOrNull()
            if (npcId == null) {
                logger.warn { "NPC 掉落表 key 非數字：$npcIdStr，跳過" }
                continue
            }

            val drops = dropsElement.jsonArray.map { entry ->
                val obj = entry.jsonObject
                NpcDropDef(
                    itemId = obj["itemId"]!!.jsonPrimitive.int,
                    rate = obj["rate"]!!.jsonPrimitive.int,
                    minQty = obj["minQty"]?.jsonPrimitive?.intOrNull ?: 1,
                    maxQty = obj["maxQty"]?.jsonPrimitive?.intOrNull ?: 1
                )
            }
            result[npcId] = drops
        }

        logger.info { "NPC 掉落表載入完成：${result.size} 種 NPC，共 ${result.values.sumOf { it.size }} 筆掉落" }
        return result
    }
}
