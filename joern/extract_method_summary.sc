@main def exec(cpgFile: String, functionName: String, filePath: String, outFile: String) = {
  importCpg(cpgFile)

  import java.nio.charset.StandardCharsets
  import java.nio.file.{Files, Paths}

  def clean(s: String): String = {
    Option(s).getOrElse("").replace("\t", " ").replace("\r", " ").replace("\n", " ").trim
  }

  def join(xs: Iterable[String], limit: Int = 200): String = {
    val values = xs.map(clean).filter(_.nonEmpty).toList.distinct.take(limit)
    if (values.isEmpty) "none" else values.mkString("|")
  }

  def one(s: String): String = {
    val v = clean(s)
    if (v.isEmpty) "none" else v
  }

  def jsonString(s: String): String = {
    val escaped = clean(s).flatMap {
      case '\\' => "\\\\"
      case '"' => "\\\""
      case '\b' => "\\b"
      case '\f' => "\\f"
      case '\n' => "\\n"
      case '\r' => "\\r"
      case '\t' => "\\t"
      case c if c < ' ' => f"\\u${c.toInt}%04x"
      case c => c.toString
    }
    "\"" + escaped + "\""
  }

  def jsonStringArray(xs: Iterable[String], limit: Int): String = {
    xs.map(clean).filter(_.nonEmpty).toList.distinct.take(limit).map(jsonString).mkString("[", ",", "]")
  }

  def lineNumberJson(n: Option[Int]): String = n.map(_.toString).getOrElse("null")

  def nameOnly(name: String): String = {
    clean(name).split("[.:]").lastOption.getOrElse(clean(name))
  }

  def isSensitiveCall(name: String): Boolean = {
    val n = nameOnly(name)

    Set(
      "gets",
      "strcpy",
      "strncpy",
      "strcat",
      "strncat",
      "sprintf",
      "vsprintf",
      "scanf",
      "sscanf",
      "fscanf",
      "memcpy",
      "memmove",
      "memset",
      "malloc",
      "calloc",
      "realloc",
      "free",
      "read",
      "recv",
      "recvfrom",
      "fread"
    ).contains(n)
  }

  def hasSecurityWord(s: String): Boolean = {
    val x = clean(s).toLowerCase
    List(
      "alloc",
      "auth",
      "bound",
      "buf",
      "copy",
      "decode",
      "free",
      "len",
      "limit",
      "mem",
      "packet",
      "parse",
      "read",
      "recv",
      "size",
      "str",
      "user",
      "valid",
      "version",
      "write"
    ).exists(x.contains)
  }

  def hasPointerOrIndexUse(s: String): Boolean = {
    val x = clean(s)
    x.contains("->") || x.contains("[") || x.contains("*") || x.contains("&")
  }

  val byName =
    if (functionName.trim.nonEmpty) {
      cpg.method.filter(m => m.name == functionName).l
    } else {
      List()
    }

  val byFile =
    if (filePath.trim.nonEmpty) {
      byName.filter { m =>
        val f = Option(m.filename).getOrElse("")
        f.endsWith(filePath) || f.contains(filePath)
      }
    } else {
      byName
    }

  val chosen = byFile.headOption.orElse(byName.headOption)

  val lines = chosen match {
    case None =>
      List(
        "FOUND\tfalse",
        "MATCHED_METHODS_COUNT\t" + byName.size.toString
      )

    case Some(mm) =>
      val mt = cpg.method.filter(m => m.id == mm.id)
      val callNames = mt.call.name.l.distinct.sorted
      val operatorNames = callNames.filter(_.startsWith("<operator>")).distinct.sorted
      val controls = mt.controlStructure.code.l.distinct
      val params = mt.parameter.map(p => s"${p.name}:${p.typeFullName}").l.distinct
      val locals = mt.local.map(x => s"${x.name}:${x.typeFullName}").l.distinct
      val returnType = mt.methodReturn.typeFullName.l.headOption.getOrElse("")
      val lineNumber = mm.lineNumber.map(_.toString).getOrElse("")

      def guardsForCall(callCode: String, callName: String): List[String] = {
        val code = clean(callCode)
        val name = clean(callName)

        mt.controlStructure
          .filter { cs =>
            val controlCode = clean(cs.code)
            code.nonEmpty && controlCode.contains(code) ||
            name.nonEmpty && controlCode.contains(name + "(")
          }
          .code
          .l
          .distinct
          .take(2)
      }

      def selectedCallReason(name: String, code: String, args: List[String]): String = {
        if (isSensitiveCall(name)) {
          "sensitive_api"
        } else if (hasSecurityWord(name) || hasSecurityWord(code) || args.exists(hasSecurityWord)) {
          "security_relevant_name_or_argument"
        } else {
          "pointer_or_index_argument"
        }
      }

      def selectedCallScore(name: String, code: String, args: List[String]): Int = {
        val sensitive = if (isSensitiveCall(name)) 100 else 0
        val securityWords = if (hasSecurityWord(name) || hasSecurityWord(code) || args.exists(hasSecurityWord)) 35 else 0
        val pointerOrIndex = if (hasPointerOrIndexUse(code) || args.exists(hasPointerOrIndexUse)) 20 else 0
        sensitive + securityWords + pointerOrIndex
      }

      val selectedCallRows = mt.call
        .filter(c => !c.name.startsWith("<operator>"))
        .l
        .map { c =>
          val args = c.argument.code.l.distinct.take(6)
          (c, args, selectedCallScore(c.name, c.code, args))
        }
        .filter { case (_, _, score) => score >= 20 }
        .sortBy { case (c, _, score) => (-score, c.lineNumber.getOrElse(0)) }
        .take(6)

      val selectedCallsJson = selectedCallRows.map { case (c, args, _) =>
        val fields = List(
          "\"callee\":" + jsonString(c.name),
          "\"line_number\":" + lineNumberJson(c.lineNumber),
          "\"code\":" + jsonString(c.code),
          "\"arguments\":" + jsonStringArray(args, 6),
          "\"guard_context\":" + jsonStringArray(guardsForCall(c.code, c.name), 2),
          "\"reason\":" + jsonString(selectedCallReason(c.name, c.code, args))
        )
        fields.mkString("{", ",", "}")
      }.mkString("[", ",", "]")

      def callerScore(code: String, args: List[String], guards: List[String]): Int = {
        val securityWords = if (hasSecurityWord(code) || args.exists(hasSecurityWord)) 35 else 0
        val pointerOrIndex = if (hasPointerOrIndexUse(code) || args.exists(hasPointerOrIndexUse)) 20 else 0
        val guarded = if (guards.nonEmpty) 10 else 0
        securityWords + pointerOrIndex + guarded
      }

      val incomingCallRows = mt.callIn.l
        .map { c =>
          val caller = Option(c.method)
          val args = c.argument.code.l.distinct.take(6)
          val guards = caller
            .map { cm =>
              val code = clean(c.code)
              val name = clean(c.name)

              cpg.method
                .filter(m => m.id == cm.id)
                .controlStructure
                .filter { cs =>
                  val controlCode = clean(cs.code)
                  code.nonEmpty && controlCode.contains(code) ||
                  name.nonEmpty && controlCode.contains(name + "(")
                }
                .code
                .l
                .distinct
                .take(2)
            }
            .getOrElse(List())
          (c, caller, args, guards, callerScore(c.code, args, guards))
        }
        .filter { case (_, _, _, _, score) => score >= 20 }
        .sortBy { case (c, _, _, _, score) => (-score, c.lineNumber.getOrElse(0)) }
        .take(3)

      val callerContextsJson = incomingCallRows.map { case (c, caller, args, guards, _) =>
        val fields = List(
          "\"caller\":" + jsonString(caller.map(_.fullName).getOrElse("unknown")),
          "\"caller_file\":" + caller.map(cm => jsonString(cm.filename)).getOrElse("null"),
          "\"line_number\":" + lineNumberJson(c.lineNumber),
          "\"code\":" + jsonString(c.code),
          "\"arguments\":" + jsonStringArray(args, 6),
          "\"guard_context\":" + jsonStringArray(guards, 2)
        )
        fields.mkString("{", ",", "}")
      }.mkString("[", ",", "]")

      List(
        "FOUND\ttrue",
        "MATCHED_METHODS_COUNT\t" + byName.size.toString,
        "METHOD_FULL_NAME\t" + one(mm.fullName),
        "METHOD_FILE\t" + one(mm.filename),
        "LINE_NUMBER\t" + one(lineNumber),
        "RETURN_TYPE\t" + one(returnType),
        "PARAMETERS\t" + join(params),
        "LOCAL_TYPES\t" + join(locals),
        "CALLS\t" + join(callNames),
        "SELECTED_CALLS_JSON\t" + selectedCallsJson,
        "CALLER_CONTEXTS_JSON\t" + callerContextsJson,
        "OPERATORS\t" + join(operatorNames),
        "CONTROL_STRUCTURES\t" + join(controls, 80),
        "CONTROL_STRUCTURE_COUNT\t" + controls.size.toString
      )
  }

  Files.write(Paths.get(outFile), (lines.mkString("\n") + "\n").getBytes(StandardCharsets.UTF_8))
}
